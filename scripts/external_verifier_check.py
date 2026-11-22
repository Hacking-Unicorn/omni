#!/usr/bin/env python3
"""
Omni Seed-Shrink (Phase C2) — Independent External Verifier Trace Checker
`scripts/external_verifier_check.py`

A zero-dependency, separately-maintained Python 3 implementation of Omni's
binary token-stream framing and core static verifier rules (MOD_BEGIN/MOD_END,
opcode bounds, taint flow to RET, and LINEAR_REFLECT 1012). Independently
verifies every trace in `spec/verifier_traces/pure_verifier_golden_traces.json`
without invoking Rust or Pure-Omni.
"""
import json
import pathlib
import sys

TOK_MOD_BEGIN = 0x7EE0
TOK_FN_DEF = 0x7EE3
TOK_INS_END = 0x7EE4
TOK_IMM_I16 = 0x7EE5
TOK_IMM_I32 = 0x7EE6
TOK_ENG_SPAN = 0x7EEA
TOK_SYM_FN = 0x7EEC
TOK_FIELD_KV = 0x7EEE
TOK_MOD_END = 0x7EEF
REG_BASE, REG_MAX = 0x7E00, 0x7EBF
PAR_BASE, PAR_MAX = 0x7EC0, 0x7EDF
OP_BASE, OP_MAX = 0x7F00, 0x7F7F
OP_ARENA = 0x7F01
OP_CALL = 0x7F48
OP_RET = 0x7F4A
TAINT_UNTRUSTED = 0x7F91  # 32657


def skip_operand(words, pos):
    tok = words[pos]
    if tok in (TOK_IMM_I16, 0x7EEB, TOK_SYM_FN, 0x7EED, TOK_FIELD_KV):
        return pos + 2
    if tok == TOK_IMM_I32:
        return pos + 3
    if tok == TOK_ENG_SPAN:
        if pos + 1 >= len(words):
            return len(words) + 1
        return pos + 2 + words[pos + 1]
    return pos + 1


def verify_stream(words):
    n = len(words)
    if n < 2 or words[0] != TOK_MOD_BEGIN:
        return False, "DECODE_ERROR"
    # Pre-scan module function table: fn_idx -> param_cnt
    fn_arities = {}
    p_scan = 1
    while p_scan < n:
        tag = words[p_scan]
        if tag == TOK_MOD_END:
            break
        if tag != TOK_FN_DEF or p_scan + 3 > n:
            return False, "DECODE_ERROR"
        fid = words[p_scan + 1]
        pcnt = words[p_scan + 2]
        hdr_end = p_scan + 3 + pcnt * 3 + 3
        if hdr_end > n:
            return False, "DECODE_ERROR"
        fn_arities[fid] = pcnt
        icnt = words[hdr_end - 1]
        c = hdr_end
        for _ in range(icnt):
            if c >= n:
                return False, "DECODE_ERROR"
            if REG_BASE <= words[c] <= REG_MAX:
                if c + 4 > n:
                    return False, "DECODE_ERROR"
                c += 4
            else:
                c += 1
            while c < n and words[c] != TOK_INS_END:
                c = skip_operand(words, c)
            if c >= n or words[c] != TOK_INS_END:
                return False, "DECODE_ERROR"
            c += 1
        p_scan = c

    pos = 1
    saw_end = False
    while pos < n:
        tag = words[pos]
        if tag == TOK_MOD_END:
            saw_end = True
            pos += 1
            break
        if tag != TOK_FN_DEF or pos + 3 > n:
            return False, "DECODE_ERROR"
        pcnt = words[pos + 2]
        hdr_end = pos + 3 + pcnt * 3 + 3
        if hdr_end > n:
            return False, "DECODE_ERROR"
        tainted = set()
        live_heap = set()
        for pi in range(pcnt):
            p_tok = words[pos + 3 + pi * 3]
            p_taint = words[pos + 3 + pi * 3 + 2]
            if p_taint == TAINT_UNTRUSTED:
                tainted.add(p_tok)
        ins_cnt = words[hdr_end - 1]
        cur = hdr_end
        for _ in range(ins_cnt):
            if cur >= n:
                return False, "DECODE_ERROR"
            dest = None
            if REG_BASE <= words[cur] <= REG_MAX:
                if cur + 4 > n:
                    return False, "DECODE_ERROR"
                dest = words[cur]
                if words[cur + 2] == TAINT_UNTRUSTED:
                    tainted.add(dest)
                op = words[cur + 3]
                cur += 4
            else:
                op = words[cur]
                cur += 1
            if not (OP_BASE <= op <= OP_MAX):
                return False, "DECODE_ERROR"
            ops = []
            op_positions = []
            while cur < n and words[cur] != TOK_INS_END:
                op_positions.append(cur)
                ops.append(words[cur])
                cur = skip_operand(words, cur)
            if cur >= n or words[cur] != TOK_INS_END:
                return False, "DECODE_ERROR"
            cur += 1
            if op == OP_ARENA and dest is not None:
                live_heap.add(dest)
            if op == OP_CALL:
                for o in ops:
                    if o in live_heap:
                        return False, "LINEAR_REFLECT"
                if not ops or ops[0] != TOK_SYM_FN or op_positions[0] + 1 >= n:
                    return False, "BAD_CALL_TARGET"
                callee_id = words[op_positions[0] + 1]
                if callee_id not in fn_arities:
                    return False, "BAD_CALL_TARGET"
                if (len(ops) - 1) != fn_arities[callee_id]:
                    return False, "CALL_ARITY_MISMATCH"
            if op == OP_RET:
                for o in ops:
                    if o in tainted:
                        return False, "TAINT_SINK"
            if dest is not None and any(o in tainted for o in ops):
                tainted.add(dest)
        pos = cur
    if not saw_end or pos != n:
        return False, "DECODE_ERROR"
    return True, None


def main():
    trace_path = pathlib.Path(__file__).resolve().parent.parent / "spec/verifier_traces/pure_verifier_golden_traces.json"
    data = json.loads(trace_path.read_text(encoding="utf-8"))
    passed = 0
    for case in data["cases"]:
        ok, code = verify_stream(case["u16_words"])
        if ok != case["expected_pass"] or code != case["expected_code"]:
            print(
                f"[C2 EXTERNAL VERIFIER FAIL] {case['id']}: got ({ok}, {code}), "
                f"want ({case['expected_pass']}, {case['expected_code']})",
                file=sys.stderr,
            )
            sys.exit(1)
        passed += 1
    print(f"[C2 EXTERNAL VERIFIER PASS] {passed}/{len(data['cases'])} published verifier traces matched independently (Python 3 oracle)")


if __name__ == "__main__":
    main()
