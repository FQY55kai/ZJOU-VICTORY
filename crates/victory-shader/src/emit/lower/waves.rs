//! Lane and wave analysis: which lanes a value needs, whether a mask is
//! read as data, and which programs can be run a wave at a time.
//!
//! A child module of [`super`] so it shares the lowering pass's types; the
//! pass itself calls in here before it emits anything.

use super::*;
/// Lane masks read as numbers (`s_bcnt1`, `s_ff1`, `v_mbcnt`, scalar
/// compares, bit tests) are ballots of the lanes' booleans, taken where the
/// mask is written: structurisation's `active` masks hold only lanes that
/// reach that block, so the ballot misses none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaskData {
    /// Compute: host lane n is console lane n (the program declares the
    /// console's wave size), so ballots are the console's masks bit for bit
    /// and any use is exact.
    Console,
    /// Pixel and vertex programs: ballots over the host's wave, whatever its
    /// size. Only uses that mean the same on any wave are allowed: counts,
    /// the first lane, the count below a lane, equality (the HLSL wave
    /// operations the console compiler expands). Lane numbers stay host
    /// lanes throughout (`v_mbcnt`, `v_readlane` at a computed lane).
    Host,
    /// Mesh programs: their lanes and waves (LDS per console wave) are the
    /// console's but no host wave size can be declared.
    None,
}

/// What a value holding a lane mask read as a number stops with.
pub(crate) const LANE_MASK: Why = "a lane mask read as data";
/// A mask with lanes outside EXEC where it is written, read as a number where the lanes that
/// run that block are not all of the wave.
pub(crate) const MASK_ELSEWHERE: Why = "a lane mask holding lanes elsewhere read as data";

/// Instructions that read a lane mask (not EXEC implicitly, not a lane's
/// bit) as a number.
/// Per IR block: every lane of the wave that started the program runs it – no per-lane branch
/// separates it from the entry (it lies in no lane branch's region: the blocks reachable from
/// the branch that do not post-dominate it). None with an indirect exit (not followed).
pub(crate) fn all_lane_blocks(st: &Structured) -> Option<Vec<bool>> {
    let n = st.exits.len();
    let mut all = vec![true; n];
    for (_, region) in lane_regions(st)? {
        for b in (0..n).filter(|&b| region[b]) {
            all[b] = false;
        }
    }
    Some(all)
}

/// Every per-lane branch (by its block) with its region: the blocks reachable from it that do
/// not post-dominate it, which only some lanes run. Not the branches the wave takes together
/// ([`Structured::wide`]), nor those on scalar conditions. None with an indirect exit.
pub(crate) fn lane_regions(st: &Structured) -> Option<Vec<(usize, Vec<bool>)>> {
    let n = st.exits.len();
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (b, e) in st.exits.iter().enumerate() {
        match *e {
            LaneExit::Jump(t) => succ[b].push(t),
            LaneExit::Branch { taken, not_taken, .. } => {
                succ[b].push(taken);
                succ[b].push(not_taken);
            }
            LaneExit::End => {}
            LaneExit::Indirect => return None,
        }
    }
    // post-dominators (bitsets; a virtual exit after every End)
    let words = n.div_ceil(64);
    let full = vec![u64::MAX; words];
    let mut pdom: Vec<Vec<u64>> = (0..n).map(|b| if succ[b].is_empty() { let mut v = vec![0; words]; v[b / 64] |= 1 << (b % 64); v } else { full.clone() }).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for b in (0..n).rev() {
            if succ[b].is_empty() {
                continue;
            }
            let mut v = full.clone();
            for &t in &succ[b] {
                for w in 0..words {
                    v[w] &= pdom[t][w];
                }
            }
            v[b / 64] |= 1 << (b % 64);
            if v != pdom[b] {
                pdom[b] = v;
                changed = true;
            }
        }
    }
    let mut regions = Vec::new();
    for (d, e) in st.exits.iter().enumerate() {
        let LaneExit::Branch { when, .. } = *e else { continue };
        if st.wide.contains(&d) || st.bools.support(when).iter().all(|a| matches!(a, Atom::Uniform(_))) {
            continue; // a wave-wide or uniform branch: all lanes go the same way
        }
        // blocks reachable from the lane branch that do not post-dominate it
        let mut seen = vec![false; n];
        let mut stack: Vec<usize> = succ[d].clone();
        while let Some(b) = stack.pop() {
            if seen[b] || pdom[d][b / 64] >> (b % 64) & 1 == 1 {
                continue;
            }
            seen[b] = true;
            stack.extend(succ[b].iter().copied());
        }
        regions.push((d, seen));
    }
    Some(regions)
}

/// The per-lane branches (by block) whose regions hold a block of `needs` (blocks where two
/// host waves must meet, [`take_needs`]): the ones to take wave-wide
/// ([`crate::structure::structure_wide`]) so that every lane runs those blocks. None if one
/// of them is a branch whose wave-wide meaning is not known ([`Structured::wave_any`]).
pub(crate) fn branches_around(st: &Structured, needs: &[usize]) -> Option<Vec<usize>> {
    let mut out = Vec::new();
    for (d, region) in lane_regions(st)? {
        if needs.iter().any(|&b| region.get(b).copied().unwrap_or(false)) {
            st.wave_any.get(d).copied().flatten()?;
            out.push(d);
        }
    }
    Some(out)
}

/// Whether a refusal is of code that needs every lane of the wave where it stands (two host
/// waves meeting there), which the other forms of the program may give it.
pub(crate) fn needs_every_lane(what: &str) -> bool {
    what.ends_with("where not every lane runs") || what == MASK_ELSEWHERE
}

/// The blocks where the last lowering on this thread wanted every lane and did not have it
/// (its "... where not every lane runs" refusals), which it forgets.
pub(crate) fn take_needs() -> Vec<usize> {
    NEEDS.take()
}

pub(crate) fn need(block: usize) {
    NEEDS.with_borrow_mut(|n| {
        if !n.contains(&block) {
            n.push(block);
        }
    });
}

/// VGPR values that are the same in every lane holding them (`all` is [`all_lane_blocks`]):
/// written by an instruction every arriving lane runs (no guard), lane by lane, from scalars,
/// constants and such values; or loaded from an address made of them. The compiler keeps
/// wave-uniform data in VGPRs and fetches it with `v_readfirstlane`.
/// shortcut: a value joined by a phi counts only where every predecessor is a block all lanes
/// run (no lane branch can have split the arriving lanes); loop-carried values never do.
pub(crate) fn lane_uniform(p: &Program, st: &Structured, all: &[bool]) -> HashSet<Value> {
    let is_mask = |v: Value| st.masks.contains_key(&v) || st.highs.contains_key(&v);
    let mut same: HashSet<Value> = HashSet::new();
    for b in crate::ir::reverse_postorder(&p.blocks) {
        let block = &p.blocks[b];
        let whole = block.preds.iter().all(|&q| all.get(q).copied().unwrap_or(false));
        for phi in &block.phis {
            if matches!(phi.reg, Reg::V(_)) && whole && phi.args.iter().all(|a| same.contains(a)) {
                same.insert(phi.value);
            }
        }
        for (i, inst) in block.insts.iter().enumerate() {
            let name = inst.name();
            let ops: Vec<Operand> = match &inst.fields {
                Fields::Valu(v)
                    if v.dpp.is_none()
                        && v.sdwa.is_none()
                        && !name.starts_with("v_mbcnt")
                        && !name.starts_with("v_cmpx")
                        && !name.contains("lane") =>
                {
                    v.src.clone()
                }
                _ => continue,
            };
            if st.guards[b][i].is_some() {
                continue; // lanes outside EXEC keep another value
            }
            let alike = ops.iter().all(|op| match *op {
                Operand::Vector { reg, .. } => {
                    // every dword of the operand the instruction reads from that register on
                    let read: Vec<Value> =
                        inst.reads.iter().filter(|r| matches!(r.0, Reg::V(x) if x >= reg)).map(|r| r.1).collect();
                    !read.is_empty() && read.iter().all(|x| same.contains(x))
                }
                // a scalar is one value; a lane mask is not, unless a compare of such values made it
                Operand::Scalar { code, .. } if code != NULL => {
                    inst.read(Reg::S(code)).is_none_or(|x| !is_mask(x) || same.contains(&x))
                }
                _ => true,
            });
            if alike {
                same.extend(inst.writes.iter().filter(|w| w.0 != Reg::Scc).map(|w| w.1));
            }
        }
    }
    same
}

/// What a register value holds, for [`wave_local`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wave {
    /// Nothing that depends on how the threads are divided into waves.
    Plain,
    /// The number of the wave's first lane in a mask (`s_ff1`).
    First,
    /// A reduction over the wave's lanes: a count, a sum, a minimum...
    Total(super::super::wave::Op),
    /// A count or sum over the lanes below this one.
    Prefix,
    /// Derived from what a first-lane atomic returned: the slots the wave reserved.
    Base,
    /// A mix the idiom does not make.
    Mixed,
}

/// Whether a console wave64 program may run with each host wave as a wave of its own
/// ([`with_local_waves`]): its wave-wide values are used only as the allocation idiom uses
/// them, which does not care how the threads are divided into waves. A first-lane number only
/// picks the first lane; a count or sum only goes into that lane's atomic on a shared counter;
/// a prefix only goes onto the base that atomic returned; the first lane does nothing else.
/// Anything else done with them takes one wave to be the whole thread group (a tile's result,
/// say) and is refused, with the offset and what was found.
/// Lane masks read as numbers by other instructions and lane reads outside whole-wave sections
/// are stopped by the lowering itself.
// shortcut: `v_readfirstlane` of a value that differs between lanes is taken as it is (this
// host wave's first lane), and a guarded write of a plain value over a wave-dependent one is
// the only mix of old and new lanes that is looked at.
pub(crate) fn wave_local(p: &Program, st: &Structured) -> Result<(), (usize, &'static str)> {
    use super::super::wave::{Op, Shape};
    const FIRST: Why = "a wave's first-lane number used as data";
    const TOTAL: Why = "a count or sum over the wave used as data";
    const PREFIX: Why = "a count or sum over the lanes below used without a reserved base";
    const MIXED: Why = "a value that depends on the wave on some paths only";
    const EFFECT: Why = "a write by the wave's first lane other than its reservation";
    fn of(kind: &HashMap<Value, Wave>, v: Option<Value>) -> Wave {
        v.and_then(|v| kind.get(&v).copied()).unwrap_or(Wave::Plain)
    }
    fn put(kind: &mut HashMap<Value, Wave>, v: Value, k: Wave) {
        if k == Wave::Plain {
            kind.remove(&v);
        } else {
            kind.insert(v, k);
        }
    }
    let wild = |k: Wave| !matches!(k, Wave::Plain | Wave::Base);
    let why = |k: Wave| match k {
        Wave::First => FIRST,
        Wave::Total(_) => TOTAL,
        Wave::Prefix => PREFIX,
        _ => MIXED,
    };
    // a mask (by its low dword) under which only a wave's first lane runs
    let first_mask = |firsts: &HashSet<Value>, m: Option<Value>| {
        m.and_then(|m| st.masks.get(&m)).is_some_and(|&b| {
            st.bools
                .support(b)
                .iter()
                .any(|a| matches!(a, Atom::Lane(v) if firsts.contains(v)))
        })
    };
    // One instruction outside a section: the kind of what it writes, and whether it is a
    // first lane's reservation.
    let step = |inst: &Inst, kind: &HashMap<Value, Wave>, firsts: &mut HashSet<Value>| -> Result<(Wave, bool), Why> {
        let name = inst.name();
        let val = |op: &Operand| match *op {
            Operand::Vector { reg, .. } => inst.read(Reg::V(reg)),
            Operand::Scalar { code, .. } if code != NULL => inst.read(Reg::S(code)),
            _ => None,
        };
        let ops: Vec<Operand> = match &inst.fields {
            Fields::Valu(v) => v.src.clone(),
            Fields::Sop { dst, src, .. } => {
                let mut o = src.clone();
                if matches!(name, "s_addk_i32" | "s_mulk_i32") {
                    o.extend(dst.iter().copied());
                }
                o
            }
            Fields::Buffer { data, .. } if name.contains("_atomic_") || name.contains("_store_") => {
                data.iter().copied().collect()
            }
            _ => Vec::new(),
        };
        let kinds: Vec<Wave> = ops.iter().map(|o| of(kind, val(o))).collect();
        // Everything else it reads (addresses, descriptors, SCC, the high dwords of wide
        // operands) must not depend on the wave. Not EXEC, and not the old value of a VGPR it
        // writes, which only the lanes outside EXEC keep.
        let named: Vec<Value> = ops.iter().filter_map(&val).collect();
        for &(r, v) in &inst.reads {
            let kept = matches!(r, Reg::V(_)) && inst.writes.iter().any(|w| w.0 == r);
            if matches!(r, Reg::S(c) if c == EXEC || c == EXEC + 1) || kept || named.contains(&v) {
                continue;
            }
            let k = of(kind, Some(v));
            if wild(k) {
                return Err(why(k));
            }
        }
        let w = kinds.iter().copied().find(|&k| wild(k)).unwrap_or(Wave::Mixed);
        let sum = |k: Wave| k == Wave::Total(Op::AddU);
        let is_mask = |o: &Operand| {
            val(o).is_some_and(|v| st.masks.contains_key(&v) || st.highs.contains_key(&v))
        };
        let mut reserved = false;
        let out = match (name, &kinds[..]) {
            ("s_ff1_i32_b64" | "s_ff1_i32_b32", _) if ops.first().is_some_and(is_mask) => Wave::First,
            ("s_bcnt1_i32_b64" | "s_bcnt1_i32_b32", _) if ops.first().is_some_and(is_mask) => {
                Wave::Total(Op::AddU)
            }
            // bits of a mask below the lane; with -1 for the mask, the lane's number
            ("v_mbcnt_lo_u32_b32" | "v_mbcnt_hi_u32_b32", [_, b]) => {
                match (matches!(ops[0], Operand::Scalar { .. }), *b) {
                    (_, Wave::Prefix) | (true, Wave::Plain | Wave::Base) => Wave::Prefix,
                    (false, Wave::Plain | Wave::Base) => Wave::Plain,
                    (_, k) => return Err(why(k)),
                }
            }
            // nothing wave-dependent goes in; what derives from a reserved base stays marked
            _ if !kinds.iter().any(|&k| wild(k)) => {
                let derives = matches!(inst.fields, Fields::Valu(_) | Fields::Sop { .. });
                if derives && kinds.contains(&Wave::Base) { Wave::Base } else { Wave::Plain }
            }
            ("v_mov_b32" | "s_mov_b32", [k]) => *k,
            ("v_cmp_eq_i32" | "v_cmp_eq_u32" | "v_cmpx_eq_i32" | "v_cmpx_eq_u32", [a, b]) => {
                // the lane whose own number is the first lane's
                let lane = match (*a, *b) {
                    (Wave::First, k) if !wild(k) => &ops[1],
                    (k, Wave::First) if !wild(k) => &ops[0],
                    _ => return Err(why(w)),
                };
                if !matches!(lane, Operand::Vector { .. }) {
                    return Err(FIRST);
                }
                let mask = match &inst.fields {
                    _ if name.starts_with("v_cmpx") => inst.write(Reg::S(EXEC)),
                    Fields::Valu(v) => match v.dst {
                        Some(Operand::Scalar { code, .. }) => inst.write(Reg::S(code)),
                        _ => None,
                    },
                    _ => None,
                };
                firsts.extend(mask);
                Wave::Plain
            }
            ("v_add_nc_u32" | "s_add_i32", [a, b]) => match (*a, *b) {
                (Wave::Prefix, Wave::Base) | (Wave::Base, Wave::Prefix) => Wave::Base,
                (Wave::Prefix, Wave::Plain | Wave::Prefix) | (Wave::Plain, Wave::Prefix) => Wave::Prefix,
                (x, y) if sum(x) && sum(y) => x,
                _ => return Err(why(w)),
            },
            ("v_sub_nc_u32" | "s_sub_i32", [a, b]) => match (*a, *b) {
                (Wave::Base, Wave::Prefix) => Wave::Base,
                (Wave::Prefix, Wave::Plain) => Wave::Prefix,
                (Wave::Plain, y) if sum(y) && matches!(ops[0], Operand::Const(128)) => y,
                _ => return Err(why(w)),
            },
            ("s_mul_i32" | "v_mul_lo_u32" | "v_mul_u32_u24", [a, b]) => match (*a, *b) {
                (x, Wave::Plain) | (Wave::Plain, x) if sum(x) || x == Wave::Prefix => x,
                _ => return Err(why(w)),
            },
            ("s_lshl_b32", [x, Wave::Plain]) | ("v_lshlrev_b32", [Wave::Plain, x])
                if sum(*x) || *x == Wave::Prefix =>
            {
                *x
            }
            ("v_lshl_add_u32", [Wave::Prefix, Wave::Plain, c]) if !wild(*c) => {
                if *c == Wave::Base { Wave::Base } else { Wave::Prefix }
            }
            // the reservation: the first lane adds its wave's total to a shared counter
            (_, [Wave::Total(op)]) if name.contains("_atomic_") => {
                let want = match op {
                    Op::AddU => "buffer_atomic_add",
                    Op::MinU => "buffer_atomic_umin",
                    Op::MaxU => "buffer_atomic_umax",
                    Op::MinI => "buffer_atomic_smin",
                    Op::MaxI => "buffer_atomic_smax",
                    Op::Or => "buffer_atomic_or",
                    Op::And => "buffer_atomic_and",
                    Op::Xor => "buffer_atomic_xor",
                    _ => "",
                };
                if name != want || !first_mask(firsts, inst.read(Reg::S(EXEC))) {
                    return Err(TOTAL);
                }
                reserved = true;
                Wave::Base
            }
            _ => return Err(why(w)),
        };
        Ok((out, reserved))
    };
    let sections: Vec<_> = (0..p.blocks.len()).map(|b| super::super::wave::sections(p, b)).collect();
    let order = crate::ir::reverse_postorder(&p.blocks);
    let pass = |kind: &mut HashMap<Value, Wave>, firsts: &mut HashSet<Value>, check: bool| -> Result<(), (usize, Why)> {
        for &b in &order {
            let block = &p.blocks[b];
            for phi in &block.phis {
                let mut k = Wave::Plain;
                for (n, &a) in phi.args.iter().enumerate() {
                    let x = of(kind, Some(a));
                    k = match (k, x) {
                        _ if n == 0 => x,
                        (k, x) if k == x => k,
                        (Wave::Plain, m @ (Wave::Base | Wave::Prefix))
                        | (m @ (Wave::Base | Wave::Prefix), Wave::Plain) => m,
                        _ => Wave::Mixed,
                    };
                }
                put(kind, phi.value, k);
            }
            for (i, inst) in block.insts.iter().enumerate() {
                // inside a whole-wave section: what the simulation found each value to be
                if let Some((_, sec)) = sections[b].iter().find(|(s, sec)| i > **s && i < sec.end) {
                    for &(_, w) in &inst.writes {
                        let k = match sec.shapes.get(&w) {
                            None => Wave::Mixed,
                            Some(&Shape::K(_)) => Wave::Plain,
                            Some(&Shape::Value(v)) => of(kind, Some(v)),
                            Some(&Shape::Masked(src, _, _)) => of(kind, Some(src)),
                            Some(&Shape::Active(op, src, mask, _)) => match of(kind, Some(src)) {
                                Wave::Plain | Wave::Base => Wave::Total(op),
                                // the first lane's own sum of a total is that total
                                Wave::Total(o) if o == op && first_mask(firsts, Some(mask)) => Wave::Total(op),
                                _ => Wave::Mixed,
                            },
                            Some(&Shape::Prefix(Op::AddU, src, mask, _, _)) => match of(kind, Some(src)) {
                                Wave::Plain | Wave::Base => Wave::Prefix,
                                Wave::Total(Op::AddU) if first_mask(firsts, Some(mask)) => Wave::Prefix,
                                _ => Wave::Mixed,
                            },
                            Some(_) => Wave::Mixed,
                        };
                        put(kind, w, k);
                    }
                    continue;
                }
                let name = inst.name();
                if name.starts_with("s_cbranch") {
                    continue; // with the block's exit, below
                }
                let (k, reserved) = match step(inst, kind, firsts) {
                    Ok(r) => r,
                    Err(why) if check => return Err((inst.machine.offset, why)),
                    Err(_) => (Wave::Mixed, false),
                };
                // the same test as `Lower::inst`'s `effect`
                let effect = matches!(inst.fields, Fields::Exp { .. })
                    || matches!(inst.fields, Fields::Flat { .. }) && !name.starts_with("global_load_")
                    || name.contains("store")
                    || name.contains("atomic")
                    || matches!(inst.fields, Fields::Ds { .. })
                        && !(name.starts_with("ds_read")
                            || name.starts_with("ds_swizzle")
                            || name.contains("permute"));
                if check && effect && !reserved && first_mask(firsts, inst.read(Reg::S(EXEC))) {
                    return Err((inst.machine.offset, EFFECT));
                }
                let guarded = st.guards[b][i].is_some();
                for &(r, w) in &inst.writes {
                    let mut kw = match r {
                        // "some lane is counted" keeps the count's kind; any other SCC is one bit of data
                        Reg::Scc if !name.starts_with("s_bcnt1") => {
                            if wild(k) { Wave::Mixed } else { Wave::Plain }
                        }
                        _ => k,
                    };
                    // a guarded write leaves the old value in the lanes outside EXEC
                    if guarded
                        && matches!(r, Reg::V(_))
                        && kw == Wave::Plain
                        && matches!(of(kind, inst.read(r)), Wave::First | Wave::Total(_) | Wave::Prefix)
                    {
                        kw = Wave::Mixed;
                    }
                    put(kind, w, kw);
                }
            }
            // a branch on "some lane is counted": fine where the structure made it the lane's
            // own bit (the mask is EXEC under a scalar condition)
            if check
                && let Some(last) = block.insts.last()
                && last.name().starts_with("s_cbranch_scc")
                && let Some(scc) = last.read(Reg::Scc)
                && wild(of(kind, Some(scc)))
            {
                let own = matches!(st.exits[b], LaneExit::Branch { when, .. }
                    if !st.bools.support(when).iter().any(|a| matches!(a, Atom::Uniform(v) if *v == scc)));
                if !own {
                    return Err((last.machine.offset, TOTAL));
                }
            }
        }
        Ok(())
    };
    let mut kind: HashMap<Value, Wave> = HashMap::new();
    let mut firsts: HashSet<Value> = HashSet::new();
    // to a fixpoint (values carried around loops), then once more to say what is wrong
    let mut settled = false;
    for _ in 0..16 {
        let before = (kind.clone(), firsts.len());
        pass(&mut kind, &mut firsts, false)?;
        if (&before.0, before.1) == (&kind, firsts.len()) {
            settled = true;
            break;
        }
    }
    if !settled {
        return Err((0, MIXED));
    }
    pass(&mut kind, &mut firsts, true)
}

/// The mode of a group barrier the translation adds (host waves meeting):
/// a wait for the group with a shared-memory fence; the fence alone with
/// `VICTORY_GROUP_SYNC=0` ([`super::super::group_sync`]).
pub(crate) fn meeting_mode() -> u32 {
    if super::super::group_sync() == 0 { 8 } else { 1 | 8 }
}

/// LDS accesses of a console wave run as two host waves, where some sit in blocks that only
/// some lanes run (`all` is [`all_lane_blocks`]): per IR block, whether the host waves meet (a
/// group barrier) at its start and before its exit. `None` if barriers cannot keep the
/// console's order.
///
/// The console runs the wave's lanes in step, so an LDS write is seen by every later read. Two
/// host waves keep that order if they meet between any two accesses of which one writes. A
/// barrier must be run by every thread, so it cannot stand in a lane branch's region; but
/// every way into and out of such a region passes a block all lanes run, and the waves meet
/// there: before the exit of an all-lane block into a region with LDS accesses, and at the
/// start of an all-lane block entered from one. Accesses in all-lane blocks get their own
/// barrier as before. What that cannot order is a write and a read inside one region (blocks
/// joined without an all-lane block between them): refused.
/// shortcut: two writes in one region are taken not to hit the same dword from both halves.
pub(crate) fn lds_meetings(p: &crate::ir::Program, st: &Structured, all: &[bool]) -> Option<Vec<(bool, bool)>> {
    let n = st.exits.len();
    let succ = |b: usize| -> Vec<usize> {
        match st.exits[b] {
            LaneExit::Jump(t) => vec![t],
            LaneExit::Branch { taken, not_taken, .. } => vec![taken, not_taken],
            LaneExit::End | LaneExit::Indirect => Vec::new(),
        }
    };
    // regions: blocks not every lane runs, joined by the edges between them
    let mut region: Vec<usize> = (0..n).collect();
    fn find(region: &mut [usize], mut b: usize) -> usize {
        while region[b] != b {
            region[b] = region[region[b]];
            b = region[b];
        }
        b
    }
    for b in 0..n {
        for t in succ(b) {
            if !all[b] && !all[t] {
                let (x, y) = (find(&mut region, b), find(&mut region, t));
                region[x] = y;
            }
        }
    }
    // per region: LDS reads, LDS writes
    let mut reads = vec![false; n];
    let mut writes = vec![false; n];
    for (b, block) in p.blocks.iter().enumerate().take(n) {
        if all[b] {
            continue;
        }
        let r = find(&mut region, b);
        for i in &block.insts {
            let name = i.name();
            if !matches!(i.fields, Fields::Ds { .. }) || name.starts_with("ds_swizzle") || name.contains("permute") {
                continue;
            }
            if name.starts_with("ds_read") {
                reads[r] = true;
            } else {
                writes[r] = true;
            }
        }
    }
    if (0..n).any(|r| reads[r] && writes[r]) {
        return None;
    }
    let mut meet = vec![(false, false); n];
    for b in 0..n {
        for t in succ(b) {
            if all[b] && !all[t] {
                let r = find(&mut region, t);
                meet[b].1 |= reads[r] || writes[r];
            } else if !all[b] && all[t] {
                let r = find(&mut region, b);
                meet[t].0 |= reads[r] || writes[r];
            }
        }
    }
    Some(meet)
}

/// Masks held as numbers or constant lane sets in guards or lane branches: the lane tests its
/// own bit by its console lane.
pub(crate) fn lane_constant_masks(st: &Structured) -> bool {
    let whens = st.exits.iter().filter_map(|e| match *e {
        LaneExit::Branch { when, .. } => Some(when),
        _ => None,
    });
    let mut work: Vec<Bool> = st.guards.iter().flatten().flatten().copied().chain(whens).collect();
    // Through lane-mask joins too: an edge that brings a number or a constant lane set needs
    // the lane's own bit of it.
    // shortcut: masks only selects read (`v_cndmask_b32`, carries in) are not followed; such
    // a program still stops with "a constant lane mask" or "an opaque lane mask".
    let (mut seen, mut joins) = (HashSet::new(), HashSet::new());
    while let Some(g) = work.pop() {
        if !seen.insert(g) {
            continue;
        }
        for a in st.bools.support(g) {
            match a {
                Atom::Opaque(_) | Atom::Constant(_) => return true,
                Atom::Phi(id) if joins.insert(id) => {
                    if let Some(edges) = st.phi_masks.get(&id) {
                        work.extend(edges.iter().copied());
                    }
                }
                _ => {}
            }
        }
    }
    false
}

/// The widest wave the host GPU runs (0: not known – no host-specific forms); see
/// [`set_host_max_wave`].
pub(crate) static HOST_MAX_WAVE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The host GPU's `WaveLaneCountMax`: 32 lets console wave64 compute programs whose cross-lane
/// code stays inside 32-lane halves run in waves of 32 instead of being refused.
pub fn set_host_max_wave(lanes: u32) {
    HOST_MAX_WAVE.store(lanes, std::sync::atomic::Ordering::Relaxed);
}

thread_local! {
    /// See [`take_needs`].
    pub(crate) static NEEDS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    /// See [`with_local_waves`].
    pub(crate) static LOCAL_WAVES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// See [`with_local_waves_any`].
    pub(crate) static LOCAL_ANY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// As [`with_local_waves`], with every other use of a lane mask as a number
/// and every lane read and write taken per host wave too: the program runs
/// as it would on a console of 32-lane waves, lanes numbered 0..31 or
/// 32..63 by half. Right for code that only relies on what HLSL's wave
/// operations promise; a program that takes one wave to be its whole
/// thread group gets per-half results. The last resort for programs that
/// do not translate otherwise.
pub(crate) fn with_local_waves_any<R>(f: impl FnOnce() -> R) -> R {
    let was = LOCAL_ANY.replace(true);
    let r = with_local_waves(f);
    LOCAL_ANY.set(was);
    r
}

/// Runs `f` with console wave64 programs on 32-lane hosts lowered with each
/// host wave as a wave of its own ("local"): no exchange between the two
/// halves of a console wave, so no barriers and no need for every lane to
/// reach the code. What HLSL's wave operations promise holds for any
/// division of the threads into waves, and the idiom the title's compute
/// programs use them for does not care which wave a thread is in: count or
/// sum over the wave, the first lane reserves that many slots with an atomic
/// add, every lane takes the base (`WaveReadLaneFirst`) plus its prefix.
/// Each lane still gets a slot of its own; only the order differs.
/// Only that idiom's operations are lowered this way: a lane mask read as
/// a number by anything but a bit count, a first-set-bit or `v_mbcnt`, and
/// lane reads and writes outside whole-wave sections, stop the translation.
/// shortcut: where a count or sum goes is not checked. A program that uses
/// a wave-wide result as data for the whole console wave (a tile's maximum,
/// say, with one wave per thread group) gets its half's result. It is used
/// only for programs that do not translate otherwise.
pub(crate) fn with_local_waves<R>(f: impl FnOnce() -> R) -> R {
    let was = LOCAL_WAVES.replace(true);
    let r = f();
    LOCAL_WAVES.set(was);
    r
}

/// `VICTORY_F16_NEAREST=1`: the results of 16-bit float operations (`v_add_f16`,
/// `v_mul_f16`, `v_cvt_f16_f32` ...) are rounded to nearest as the console rounds them
/// ([`Lower::f16_nearest`]) instead of as the host's `f32tof16` does (toward zero on NVIDIA:
/// the last place of a result that is not exact is then one lower than the console's).
/// Off by default: it is exact but untried in a game run, and adds instructions to every
/// such operation. The wave sum of 16-bit floats ([`Lower::half_sum`]) is always exact.
pub(crate) fn f16_nearest_on() -> bool {
    #[cfg(test)]
    if let Some(on) = TEST_F16_NEAREST.get() {
        return on;
    }
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_F16_NEAREST").is_some_and(|v| v == "1"))
}

#[cfg(test)]
thread_local! {
    /// A test's choice for [`f16_nearest_on`], for its thread only.
    pub(crate) static TEST_F16_NEAREST: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// `VICTORY_WAVE_OWN_BITS=0`: a wave OR where not every lane runs is refused as before
/// ([`super::own_bits`]).
pub(crate) fn own_bits_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_WAVE_OWN_BITS").is_none_or(|v| v != "0"))
}

/// The maximum wave width selected for shader translation.
pub fn host_max_wave() -> u32 {
    #[cfg(test)]
    if let Some(lanes) = TEST_HOST_WAVE.get() {
        return lanes;
    }
    HOST_MAX_WAVE.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
thread_local! {
    /// A test's host wave size, for its thread only ([`set_host_max_wave`] is the process's).
    pub(crate) static TEST_HOST_WAVE: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

pub(crate) fn reads_mask_as_data(st: &Structured, i: &Inst) -> bool {
    let is_mask = |v: Value| st.masks.contains_key(&v) || st.highs.contains_key(&v);
    let ops: Vec<&Operand> = match &i.fields {
        // A mask operation (its result a mask) is no data use.
        Fields::Sop { .. }
            if i.writes
                .iter()
                .any(|&(r, w)| matches!(r, Reg::S(_)) && is_mask(w)) =>
        {
            return false;
        }
        Fields::Sop { src, .. } => src.iter().collect(),
        Fields::Valu(v)
            if !matches!(i.name(), "v_cndmask_b32") && !i.name().contains("_co_ci_") =>
        {
            v.src.iter().collect()
        }
        _ => return false,
    };
    ops.into_iter().any(|op| match *op {
        Operand::Scalar { code, .. } if code != NULL => i.read(Reg::S(code)).is_some_and(is_mask),
        _ => false,
    })
}

