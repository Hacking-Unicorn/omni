#![allow(dead_code, unused_imports)]
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use omni_isa::*;
use omni_ir::*;
use omni_verifier::*;
use omni_vm::*;
use omni_assembler::*;
use omni_aot::*;

use crate::ternary::*;
use crate::verifier::*;
use crate::host::*;
use crate::repair_agent::*;
use crate::harness::*;
use crate::inspect::*;
use crate::acceptance::*;
use crate::pure_omni::*;

// ==============================================================================
// 7B. OMNI SURFACE LANGUAGE (`omni-lang`) -> `.omni_asm` -> `Vec<u16>`
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SurfaceTok {
    Ident(String),
    Int(i64),
    Trit(String, i8),
    StrLit(String),
    AtIdent(String),
    Sym(String),
    Op(String),
    Hash,
}

#[derive(Clone, Debug)]
pub(crate) struct SpannedSurfaceTok {
    pub(crate) tok: SurfaceTok,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceParam {
    pub(crate) name: String,
    pub(crate) ty: String,
    pub(crate) untrusted: bool,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SurfaceFnDef {
    pub(crate) fn_id_attr: Option<u16>,
    pub(crate) cap_mask: CapMask,
    pub(crate) step_budget_attr: Option<u64>,
    pub(crate) mem_budget_attr: Option<usize>,
    pub(crate) budget_span: Option<(usize, usize)>,
    pub(crate) strict_attr: bool,
    pub(crate) is_route: bool,
    pub(crate) route_method: Option<String>,
    pub(crate) route_path: Option<String>,
    pub(crate) name: String,
    pub(crate) fn_line: usize,
    pub(crate) fn_col: usize,
    pub(crate) params: Vec<SurfaceParam>,
    pub(crate) ret_ty: String,
    pub(crate) ret_line: usize,
    pub(crate) ret_col: usize,
    pub(crate) body: Vec<SurfaceStmt>,
}

#[derive(Clone, Debug)]
pub(crate) enum SurfaceExpr {
    Int(i64, usize, usize),
    Trit(String, i8, usize, usize),
    StrLit(String, usize, usize),
    TritVecLit(Vec<i8>, usize, usize),
    Var(String, usize, usize),
    UnOp(String, Box<SurfaceExpr>, usize, usize),
    BinOp(String, Box<SurfaceExpr>, Box<SurfaceExpr>, usize, usize),
    Call(String, Vec<SurfaceExpr>, usize, usize),
    IfBranch(String, Box<SurfaceExpr>, Vec<SurfaceStmt>, Vec<SurfaceStmt>, usize, usize),
    MatchTrit(Box<SurfaceExpr>, Vec<SurfaceStmt>, Vec<SurfaceStmt>, Vec<SurfaceStmt>, usize, usize),
}

impl SurfaceExpr {
    pub(crate) fn span(&self) -> (usize, usize) {
        match *self {
            SurfaceExpr::Int(_, l, c)
            | SurfaceExpr::Trit(_, _, l, c)
            | SurfaceExpr::StrLit(_, l, c)
            | SurfaceExpr::TritVecLit(_, l, c)
            | SurfaceExpr::Var(_, l, c)
            | SurfaceExpr::UnOp(_, _, l, c)
            | SurfaceExpr::BinOp(_, _, _, l, c)
            | SurfaceExpr::Call(_, _, l, c)
            | SurfaceExpr::IfBranch(_, _, _, _, l, c)
            | SurfaceExpr::MatchTrit(_, _, _, _, l, c) => (l, c),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SurfaceStmt {
    Let(String, Option<(String, usize, usize)>, SurfaceExpr, usize, usize),
    Assign(String, SurfaceExpr, usize, usize),
    DoWhile(Vec<SurfaceStmt>, SurfaceExpr, usize, usize),
    While(SurfaceExpr, Vec<SurfaceStmt>, usize, usize),
    Label(u16, usize, usize),
    Goto(u16, usize, usize),
    Ret(SurfaceExpr, usize, usize),
    Expr(SurfaceExpr, usize, usize),
}

impl SurfaceStmt {
    pub(crate) fn span(&self) -> (usize, usize) {
        match *self {
            SurfaceStmt::Let(_, _, _, l, c)
            | SurfaceStmt::Assign(_, _, l, c)
            | SurfaceStmt::DoWhile(_, _, l, c)
            | SurfaceStmt::While(_, _, l, c)
            | SurfaceStmt::Label(_, l, c)
            | SurfaceStmt::Goto(_, l, c)
            | SurfaceStmt::Ret(_, l, c)
            | SurfaceStmt::Expr(_, l, c) => (l, c),
        }
    }
}

pub(crate) fn tokenize_omni_surface_inner(src: &str, strict: bool) -> Result<Vec<SpannedSurfaceTok>, String> {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    let mut toks = Vec::new();

    while i < n {
        let c = bytes[i] as char;
        if c == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            col += 1;
            continue;
        }
        if i + 1 < n && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
                col += 1;
            }
            continue;
        }
        if bytes[i] == b'#' {
            if i + 1 < n && bytes[i + 1] == b'[' {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Hash, line, col });
                i += 1;
                col += 1;
                continue;
            } else {
                while i < n && bytes[i] != b'\n' {
                    i += 1;
                    col += 1;
                }
                continue;
            }
        }
        if bytes[i] == b'"' {
            let start_line = line;
            let start_col = col;
            i += 1;
            col += 1;
            let start = i;
            while i < n && bytes[i] != b'"' && bytes[i] != b'\n' {
                i += 1;
                col += 1;
            }
            if i >= n || bytes[i] != b'"' {
                return Err(format!(
                    "error[SYNTAX_ERROR] line {}, col {}: Unterminated string literal",
                    start_line, start_col
                ));
            }
            let s = String::from(&src[start..i]);
            i += 1;
            col += 1;
            toks.push(SpannedSurfaceTok {
                tok: SurfaceTok::StrLit(s),
                line: start_line,
                col: start_col,
            });
            continue;
        }
        if bytes[i] == b'@' {
            let start_col = col;
            i += 1;
            col += 1;
            let start = i;
            while i < n && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            if start == i {
                return Err(format!(
                    "error[SYNTAX_ERROR] line {}, col {}: Expected annotation name after '@'",
                    line, start_col
                ));
            }
            toks.push(SpannedSurfaceTok {
                tok: SurfaceTok::AtIdent(String::from(&src[start..i])),
                line,
                col: start_col,
            });
            continue;
        }
        if i + 3 <= n && &src[i..i + 3] == "<=>" {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Op(String::from("<=>")), line, col });
            i += 3;
            col += 3;
            continue;
        }
        if i + 3 <= n && &src[i..i + 3] == "+1t" {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("+1"), 1), line, col });
            i += 3;
            col += 3;
            continue;
        }
        if i + 3 <= n && &src[i..i + 3] == "-1t" {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("-1t"), -1), line, col });
            i += 3;
            col += 3;
            continue;
        }
        if i + 2 <= n && &src[i..i + 2] == "0t" {
            let next_is_ident = i + 2 < n && ((bytes[i + 2] as char).is_ascii_alphanumeric() || bytes[i + 2] == b'_');
            if !next_is_ident {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("0t"), 0), line, col });
                i += 2;
                col += 2;
                continue;
            }
        }
        if i + 2 <= n {
            let two = &src[i..i + 2];
            if two == "->" || two == "=>" || two == "+=" || two == "-=" || two == "*=" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Sym(String::from(two)), line, col });
                i += 2;
                col += 2;
                continue;
            }
            if two == "==" || two == "!=" || two == "<=" || two == ">=" || two == "<<" || two == ">>" {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Op(String::from(two)), line, col });
                i += 2;
                col += 2;
                continue;
            }
        }
        if "(){}[].,:;=".contains(c) {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Sym(c.to_string()), line, col });
            i += 1;
            col += 1;
            continue;
        }
        if "+-*/%&|^~!<>".contains(c) {
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Op(c.to_string()), line, col });
            i += 1;
            col += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            let start_col = col;
            while i < n && (bytes[i] as char).is_ascii_digit() {
                i += 1;
                col += 1;
            }
            let val = src[start..i].parse::<i64>().map_err(|e| {
                format!(
                    "error[SYNTAX_ERROR] line {}, col {}: Invalid integer literal: {}",
                    line, start_col, e
                )
            })?;
            toks.push(SpannedSurfaceTok { tok: SurfaceTok::Int(val), line, col: start_col });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            let start_col = col;
            while i < n && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
                col += 1;
            }
            let ident = &src[start..i];
            if ident == "TRIT_POS" || ident == "TRIT_ZERO" || ident == "TRIT_NEG" {
                if strict {
                    return Err(format!(
                        "error[AMBIGUOUS_GRAMMAR] line {}, col {}: Non-canonical trit literal '{}'; use '+1t', '0t', or '-1t' in strict mode",
                        line, start_col, ident
                    ));
                }
                if ident == "TRIT_POS" {
                    toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("+1"), 1), line, col: start_col });
                } else if ident == "TRIT_ZERO" {
                    toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("0t"), 0), line, col: start_col });
                } else {
                    toks.push(SpannedSurfaceTok { tok: SurfaceTok::Trit(String::from("-1t"), -1), line, col: start_col });
                }
            } else {
                toks.push(SpannedSurfaceTok { tok: SurfaceTok::Ident(String::from(ident)), line, col: start_col });
            }
            continue;
        }
        return Err(format!(
            "error[SYNTAX_ERROR] line {}, col {}: Unexpected character '{}'",
            line, col, c
        ));
    }
    Ok(toks)
}

pub(crate) fn tokenize_omni_surface(src: &str) -> Result<Vec<SpannedSurfaceTok>, String> {
    tokenize_omni_surface_inner(src, false)
}

pub(crate) fn parse_surface_cap_name(name: &str, line: usize, col: usize) -> Result<CapMask, String> {
    match name.to_ascii_uppercase().as_str() {
        "PURE" => Ok(0),
        "DB_R" | "DB_READ" => Ok(cap_bit(CAP_ID_DB_R)),
        "DB_W" | "DB_WRITE" | "KV_W" => Ok(cap_bit(CAP_ID_DB_W)),
        "NET" | "HTTP" => Ok(cap_bit(CAP_ID_NET)),
        "DOM" => Ok(cap_bit(CAP_ID_DOM)),
        "AI" => Ok(cap_bit(CAP_ID_AI)),
        "VISION" | "SKILL" => Ok(cap_bit(CAP_ID_VISION)),
        "AUDIO" => Ok(cap_bit(CAP_ID_AUDIO)),
        "AUTH" => Ok(cap_bit(CAP_ID_AUTH)),
        "FS" | "FS_R" | "FS_W" => Ok(cap_bit(CAP_ID_FS)),
        "SYS" | "ENV" | "PROC" | "TIME" | "RAND" | "CRYPTO" => Ok(cap_bit(CAP_ID_SYS)),
        other => Err(format!(
            "error[MISSING_CAP] line {}, col {}: Unknown capability '{}'",
            line, col, other
        )),
    }
}

/// Checks that a hygienic surface macro body only references its formal parameters (preventing accidental caller variable capture).
pub(crate) fn check_macro_body_hygiene(
    expr: &SurfaceExpr,
    params: &[String],
    macro_name: &str,
) -> Result<(), String> {
    match expr {
        SurfaceExpr::Int(_, _, _)
        | SurfaceExpr::Trit(_, _, _, _)
        | SurfaceExpr::StrLit(_, _, _)
        | SurfaceExpr::TritVecLit(_, _, _) => Ok(()),
        SurfaceExpr::Var(v, l, c) => {
            if params.iter().any(|p| p == v) {
                Ok(())
            } else {
                Err(format!(
                    "error[UNDEF_VAR] line {}, col {}: Hygienic macro '{}' referenced unbound variable '{}'",
                    l, c, macro_name, v
                ))
            }
        }
        SurfaceExpr::UnOp(_, sub, _, _) => check_macro_body_hygiene(sub, params, macro_name),
        SurfaceExpr::BinOp(_, lhs, rhs, _, _) => {
            check_macro_body_hygiene(lhs, params, macro_name)?;
            check_macro_body_hygiene(rhs, params, macro_name)
        }
        SurfaceExpr::Call(_, args, _, _) => {
            for a in args {
                check_macro_body_hygiene(a, params, macro_name)?;
            }
            Ok(())
        }
        SurfaceExpr::IfBranch(_, cond, then_b, else_b, _, _) => {
            check_macro_body_hygiene(cond, params, macro_name)?;
            for st in then_b.iter().chain(else_b.iter()) {
                if let SurfaceStmt::Expr(e, _, _) | SurfaceStmt::Ret(e, _, _) = st {
                    check_macro_body_hygiene(e, params, macro_name)?;
                }
            }
            Ok(())
        }
        SurfaceExpr::MatchTrit(cond, neg_b, zero_b, pos_b, _, _) => {
            check_macro_body_hygiene(cond, params, macro_name)?;
            for st in neg_b.iter().chain(zero_b.iter()).chain(pos_b.iter()) {
                if let SurfaceStmt::Expr(e, _, _) | SurfaceStmt::Ret(e, _, _) = st {
                    check_macro_body_hygiene(e, params, macro_name)?;
                }
            }
            Ok(())
        }
    }
}

/// Hygienically substitutes macro arguments into a macro body expression, anchoring generated nodes to `(call_line, call_col)`.
pub(crate) fn substitute_macro_expr(
    expr: &SurfaceExpr,
    param_map: &BTreeMap<String, SurfaceExpr>,
    call_line: usize,
    call_col: usize,
) -> Result<SurfaceExpr, String> {
    match expr {
        SurfaceExpr::Int(v, _, _) => Ok(SurfaceExpr::Int(*v, call_line, call_col)),
        SurfaceExpr::Trit(s, t, _, _) => Ok(SurfaceExpr::Trit(s.clone(), *t, call_line, call_col)),
        SurfaceExpr::StrLit(s, _, _) => Ok(SurfaceExpr::StrLit(s.clone(), call_line, call_col)),
        SurfaceExpr::TritVecLit(tv, _, _) => Ok(SurfaceExpr::TritVecLit(tv.clone(), call_line, call_col)),
        SurfaceExpr::Var(v, l, c) => param_map.get(v).cloned().ok_or_else(|| {
            format!(
                "error[UNDEF_VAR] line {}, col {}: Hygienic macro referenced unbound variable '{}'",
                l, c, v
            )
        }),
        SurfaceExpr::UnOp(op, sub, _, _) => Ok(SurfaceExpr::UnOp(
            op.clone(),
            Box::new(substitute_macro_expr(sub, param_map, call_line, call_col)?),
            call_line,
            call_col,
        )),
        SurfaceExpr::BinOp(op, lhs, rhs, _, _) => Ok(SurfaceExpr::BinOp(
            op.clone(),
            Box::new(substitute_macro_expr(lhs, param_map, call_line, call_col)?),
            Box::new(substitute_macro_expr(rhs, param_map, call_line, call_col)?),
            call_line,
            call_col,
        )),
        SurfaceExpr::Call(cname, args, _, _) => {
            let mut sub_args = Vec::with_capacity(args.len());
            for a in args {
                sub_args.push(substitute_macro_expr(a, param_map, call_line, call_col)?);
            }
            Ok(SurfaceExpr::Call(cname.clone(), sub_args, call_line, call_col))
        }
        SurfaceExpr::IfBranch(kind, cond, then_b, else_b, _, _) => {
            let sub_cond = Box::new(substitute_macro_expr(cond, param_map, call_line, call_col)?);
            let mut sub_then = Vec::with_capacity(then_b.len());
            for st in then_b {
                if let SurfaceStmt::Expr(e, _, _) = st {
                    sub_then.push(SurfaceStmt::Expr(
                        substitute_macro_expr(e, param_map, call_line, call_col)?,
                        call_line,
                        call_col,
                    ));
                }
            }
            let mut sub_else = Vec::with_capacity(else_b.len());
            for st in else_b {
                if let SurfaceStmt::Expr(e, _, _) = st {
                    sub_else.push(SurfaceStmt::Expr(
                        substitute_macro_expr(e, param_map, call_line, call_col)?,
                        call_line,
                        call_col,
                    ));
                }
            }
            Ok(SurfaceExpr::IfBranch(
                kind.clone(),
                sub_cond,
                sub_then,
                sub_else,
                call_line,
                call_col,
            ))
        }
        SurfaceExpr::MatchTrit(cond, neg_b, zero_b, pos_b, _, _) => {
            let sub_cond = Box::new(substitute_macro_expr(cond, param_map, call_line, call_col)?);
            let mut sub_neg = Vec::new();
            for st in neg_b {
                if let SurfaceStmt::Expr(e, _, _) = st {
                    sub_neg.push(SurfaceStmt::Expr(
                        substitute_macro_expr(e, param_map, call_line, call_col)?,
                        call_line,
                        call_col,
                    ));
                }
            }
            let mut sub_zero = Vec::new();
            for st in zero_b {
                if let SurfaceStmt::Expr(e, _, _) = st {
                    sub_zero.push(SurfaceStmt::Expr(
                        substitute_macro_expr(e, param_map, call_line, call_col)?,
                        call_line,
                        call_col,
                    ));
                }
            }
            let mut sub_pos = Vec::new();
            for st in pos_b {
                if let SurfaceStmt::Expr(e, _, _) = st {
                    sub_pos.push(SurfaceStmt::Expr(
                        substitute_macro_expr(e, param_map, call_line, call_col)?,
                        call_line,
                        call_col,
                    ));
                }
            }
            Ok(SurfaceExpr::MatchTrit(
                sub_cond,
                sub_neg,
                sub_zero,
                sub_pos,
                call_line,
                call_col,
            ))
        }
    }
}

pub(crate) struct SurfaceParser {
    pub(crate) tokens: Vec<SpannedSurfaceTok>,
    pub(crate) pos: usize,
    pub(crate) strict_mode: bool,
    pub(crate) extra_fn_caps: CapMask,
    pub(crate) macros: BTreeMap<String, (Vec<String>, SurfaceExpr)>,
}

impl SurfaceParser {
    pub(crate) fn new(tokens: Vec<SpannedSurfaceTok>) -> Self {
        Self::new_with_strict(tokens, false)
    }

    pub(crate) fn new_with_strict(tokens: Vec<SpannedSurfaceTok>, strict_mode: bool) -> Self {
        Self {
            tokens,
            pos: 0,
            strict_mode,
            extra_fn_caps: 0,
            macros: BTreeMap::new(),
        }
    }

    pub(crate) fn current_span(&self) -> (usize, usize) {
        if let Some(t) = self.tokens.get(self.pos) {
            (t.line, t.col)
        } else if let Some(last) = self.tokens.last() {
            (last.line, last.col + 1)
        } else {
            (1, 1)
        }
    }

    pub(crate) fn peek(&self) -> Option<&SurfaceTok> {
        self.tokens.get(self.pos).map(|s| &s.tok)
    }

    pub(crate) fn next_spanned(&mut self) -> Result<SpannedSurfaceTok, String> {
        if let Some(t) = self.tokens.get(self.pos).cloned() {
            self.pos += 1;
            Ok(t)
        } else {
            let (l, c) = self.current_span();
            Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Unexpected EOF in surface program",
                l, c
            ))
        }
    }

    pub(crate) fn expect_sym(&mut self, sym: &str) -> Result<(usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Sym(ref s) if s == sym => Ok((st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected symbol '{}', got {:?}",
                st.line, st.col, sym, other
            )),
        }
    }

    pub(crate) fn expect_ident_spanned(&mut self) -> Result<(String, usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Ident(s) => Ok((s, st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected identifier, got {:?}",
                st.line, st.col, other
            )),
        }
    }

    pub(crate) fn expect_ident_kw(&mut self, kw: &str) -> Result<(usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Ident(ref s) if s == kw => Ok((st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected keyword '{}', got {:?}",
                st.line, st.col, kw, other
            )),
        }
    }

    pub(crate) fn expect_int_spanned(&mut self) -> Result<(i64, usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Int(v) => Ok((v, st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected integer literal, got {:?}",
                st.line, st.col, other
            )),
        }
    }

    pub(crate) fn consume_at_annotations(&mut self) -> Result<CapMask, String> {
        let mut mask: CapMask = 0;
        while let Some(SurfaceTok::AtIdent(tag)) = self.peek().cloned() {
            let low = tag.to_ascii_lowercase();
            if low == "cap" {
                let _ = self.next_spanned()?;
                self.expect_sym("(")?;
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                    let (cname, cl, cc) = self.expect_ident_spanned()?;
                    mask |= parse_surface_cap_name(&cname, cl, cc)?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                }
                self.expect_sym(")")?;
            } else if low == "pack5" || low == "safe" {
                let _ = self.next_spanned()?;
            } else {
                break;
            }
        }
        self.extra_fn_caps |= mask;
        Ok(mask)
    }

    pub(crate) fn parse_route_path(&mut self) -> Result<String, String> {
        if let Some(SurfaceTok::StrLit(s)) = self.peek().cloned() {
            let _ = self.next_spanned()?;
            return Ok(s);
        }
        let mut path = String::new();
        while let Some(tok) = self.peek().cloned() {
            match tok {
                SurfaceTok::Op(ref s) if s == "/" || s == "-" || s == "*" => {
                    let _ = self.next_spanned()?;
                    path.push_str(s);
                }
                SurfaceTok::Sym(ref s) if s == ":" || s == "." => {
                    let _ = self.next_spanned()?;
                    path.push_str(s);
                }
                SurfaceTok::Ident(ref s) => {
                    let _ = self.next_spanned()?;
                    path.push_str(s);
                }
                SurfaceTok::Int(v) => {
                    let _ = self.next_spanned()?;
                    path.push_str(&v.to_string());
                }
                _ => break,
            }
        }
        if path.is_empty() {
            let (l, c) = self.current_span();
            return Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected route path (e.g. /users/:id or \"/users/:id\")",
                l, c
            ));
        }
        Ok(path)
    }

    pub(crate) fn parse_module(&mut self) -> Result<(Vec<(String, usize, usize)>, Vec<SurfaceFnDef>), String> {
        let mut includes = Vec::new();
        let mut fns = Vec::new();

        while self.peek().is_some() {
            if self.peek() == Some(&SurfaceTok::Ident(String::from("include")))
                || self.peek() == Some(&SurfaceTok::Ident(String::from("mod")))
            {
                let kw_st = self.next_spanned()?;
                let kw_str = if let SurfaceTok::Ident(k) = kw_st.tok { k } else { String::from("include") };
                if self.strict_mode && kw_str == "mod" {
                    return Err(format!(
                        "error[AMBIGUOUS_GRAMMAR] line {}, col {}: Non-canonical 'mod' directive; use 'include \"...\"' in strict mode",
                        kw_st.line, kw_st.col
                    ));
                }
                let path_st = self.next_spanned()?;
                let inc_path = match path_st.tok {
                    SurfaceTok::StrLit(s) => s,
                    other => {
                        return Err(format!(
                            "error[SYNTAX_ERROR] line {}, col {}: Expected string literal after '{}', got {:?}",
                            path_st.line, path_st.col, kw_str, other
                        ));
                    }
                };
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                includes.push((inc_path, kw_st.line, kw_st.col));
                continue;
            }
            if self.peek() == Some(&SurfaceTok::Ident(String::from("macro"))) {
                let _ = self.next_spanned()?;
                let (m_name, ml, mc) = self.expect_ident_spanned()?;
                self.expect_sym("(")?;
                let mut m_params = Vec::new();
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                    let (p_name, _pl, _pc) = self.expect_ident_spanned()?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(":"))) {
                        let _ = self.next_spanned()?;
                        let _ = self.expect_ident_spanned()?;
                    }
                    m_params.push(p_name);
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                }
                self.expect_sym(")")?;
                let m_body = if self.peek() == Some(&SurfaceTok::Sym(String::from("="))) {
                    let _ = self.next_spanned()?;
                    let e = self.parse_expr()?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                        let _ = self.next_spanned()?;
                    }
                    e
                } else if self.peek() == Some(&SurfaceTok::Sym(String::from("{"))) {
                    let _ = self.next_spanned()?;
                    let e = self.parse_expr()?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                        let _ = self.next_spanned()?;
                    }
                    self.expect_sym("}")?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                        let _ = self.next_spanned()?;
                    }
                    e
                } else {
                    return Err(format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Expected '=' or '{{' in macro '{}' definition",
                        ml, mc, m_name
                    ));
                };
                check_macro_body_hygiene(&m_body, &m_params, &m_name)?;
                self.macros.insert(m_name, (m_params, m_body));
                continue;
            }
            fns.push(self.parse_fn_def()?);
        }

        if fns.is_empty() {
            let (l, c) = self.current_span();
            return Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Surface program contains no 'fn' or 'route' declarations",
                l, c
            ));
        }
        Ok((includes, fns))
    }

    pub(crate) fn parse_fn_def(&mut self) -> Result<SurfaceFnDef, String> {
        let mut fn_id_attr: Option<u16> = None;
        let mut cap_mask: CapMask = 0x0000;
        let mut step_budget_attr: Option<u64> = None;
        let mut mem_budget_attr: Option<usize> = None;
        let mut budget_span: Option<(usize, usize)> = None;
        let mut fn_strict = self.strict_mode;
        self.extra_fn_caps = 0;

        loop {
            if self.peek() == Some(&SurfaceTok::Hash) {
                let _ = self.next_spanned()?;
                self.expect_sym("[")?;
                let (attr, al, ac) = self.expect_ident_spanned()?;
                if attr == "fn_id" {
                    self.expect_sym("(")?;
                    let (id_val, il, ic) = self.expect_int_spanned()?;
                    if !(0..=15).contains(&id_val) {
                        return Err(format!(
                            "error[SYNTAX_ERROR] line {}, col {}: fn_id {} out of range 0..=15",
                            il, ic, id_val
                        ));
                    }
                    fn_id_attr = Some(id_val as u16);
                    self.expect_sym(")")?;
                } else if attr == "cap" {
                    self.expect_sym("(")?;
                    while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                        let (cname, cl, cc) = self.expect_ident_spanned()?;
                        cap_mask |= parse_surface_cap_name(&cname, cl, cc)?;
                        if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                            let _ = self.next_spanned()?;
                        }
                    }
                    self.expect_sym(")")?;
                } else if attr == "step_budget" || attr == "fuel" {
                    self.expect_sym("(")?;
                    let (b_val, bl, bc) = self.expect_int_spanned()?;
                    if b_val <= 0 {
                        return Err(format!(
                            "error[BUDGET_EXCEEDED] line {}, col {}: step_budget must be > 0, got {}",
                            bl, bc, b_val
                        ));
                    }
                    step_budget_attr = Some(b_val as u64);
                    budget_span = Some((al, ac));
                    self.expect_sym(")")?;
                } else if attr == "mem_budget" || attr == "memory_budget" {
                    self.expect_sym("(")?;
                    let (m_val, ml, mc) = self.expect_int_spanned()?;
                    if m_val <= 0 {
                        return Err(format!(
                            "error[BUDGET_EXCEEDED] line {}, col {}: mem_budget must be > 0, got {}",
                            ml, mc, m_val
                        ));
                    }
                    mem_budget_attr = Some(m_val as usize);
                    budget_span = Some((al, ac));
                    self.expect_sym(")")?;
                } else if attr == "strict" {
                    fn_strict = true;
                } else {
                    return Err(format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Unsupported surface attribute '{}'",
                        al, ac, attr
                    ));
                }
                self.expect_sym("]")?;
            } else if matches!(self.peek(), Some(SurfaceTok::AtIdent(_))) {
                cap_mask |= self.consume_at_annotations()?;
            } else {
                break;
            }
        }

        let prev_strict = self.strict_mode;
        self.strict_mode = fn_strict;

        if self.peek() == Some(&SurfaceTok::Ident(String::from("route"))) {
            let route_st = self.next_spanned()?;
            let (method, _ml, _mc) = self.expect_ident_spanned()?;
            let path = self.parse_route_path()?;
            let mut params = Vec::new();
            if self.peek() == Some(&SurfaceTok::Sym(String::from("("))) {
                let _ = self.next_spanned()?;
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                    let (p_name, _pl, _pc) = self.expect_ident_spanned()?;
                    self.expect_sym(":")?;
                    let (raw_ty, tl, tc) = self.expect_ident_spanned()?;
                    let mut untrusted = false;
                    let clean_ty = if let Some(stripped) = raw_ty.strip_prefix("untrusted_") {
                        if self.strict_mode {
                            self.strict_mode = prev_strict;
                            return Err(format!(
                                "error[AMBIGUOUS_GRAMMAR] line {}, col {}: Non-canonical taint prefix '{}'; use '{}: {} @untrusted' in strict mode",
                                tl, tc, raw_ty, p_name, stripped
                            ));
                        }
                        untrusted = true;
                        String::from(stripped)
                    } else {
                        raw_ty
                    };
                    if let Some(SurfaceTok::AtIdent(tag)) = self.peek().cloned() {
                        let st = self.next_spanned()?;
                        match tag.to_ascii_lowercase().as_str() {
                            "untrusted" => untrusted = true,
                            "safe" => untrusted = false,
                            other => {
                                self.strict_mode = prev_strict;
                                return Err(format!(
                                    "error[BAD_TYPE] line {}, col {}: Unknown taint annotation '@{}'",
                                    st.line, st.col, other
                                ));
                            }
                        }
                    }
                    params.push(SurfaceParam {
                        name: p_name,
                        ty: clean_ty,
                        untrusted,
                        line: tl,
                        col: tc,
                    });
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                }
                self.expect_sym(")")?;
            }
            if params.is_empty() {
                params.push(SurfaceParam {
                    name: String::from("req"),
                    ty: String::from("i64"),
                    untrusted: false,
                    line: route_st.line,
                    col: route_st.col,
                });
            }
            cap_mask |= self.consume_at_annotations()?;
            let (ret_ty, rl, rc) = if self.peek() == Some(&SurfaceTok::Sym(String::from("->"))) {
                let _ = self.next_spanned()?;
                self.expect_ident_spanned()?
            } else {
                (String::from("i64"), route_st.line, route_st.col)
            };
            cap_mask |= self.consume_at_annotations()?;
            let body = self.parse_block()?;
            cap_mask |= self.extra_fn_caps;
            self.strict_mode = prev_strict;

            let sanitized_path: String = path
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect();
            let route_fn_name = format!("route_{}_{}", method.to_ascii_lowercase(), sanitized_path);
            return Ok(SurfaceFnDef {
                fn_id_attr,
                cap_mask,
                step_budget_attr,
                mem_budget_attr,
                budget_span,
                strict_attr: fn_strict,
                is_route: true,
                route_method: Some(method.to_ascii_uppercase()),
                route_path: Some(path),
                name: route_fn_name,
                fn_line: route_st.line,
                fn_col: route_st.col,
                params,
                ret_ty,
                ret_line: rl,
                ret_col: rc,
                body,
            });
        }

        let (fn_kw_line, fn_kw_col) = self.expect_ident_kw("fn")?;
        let (fn_name, _fl, _fc) = self.expect_ident_spanned()?;
        self.expect_sym("(")?;
        let mut params = Vec::new();
        while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
            let (p_name, _pl, _pc) = self.expect_ident_spanned()?;
            self.expect_sym(":")?;
            let (raw_ty, tl, tc) = self.expect_ident_spanned()?;
            let mut untrusted = false;
            let clean_ty = if let Some(stripped) = raw_ty.strip_prefix("untrusted_") {
                if self.strict_mode {
                    self.strict_mode = prev_strict;
                    return Err(format!(
                        "error[AMBIGUOUS_GRAMMAR] line {}, col {}: Non-canonical taint prefix '{}'; use '{}: {} @untrusted' in strict mode",
                        tl, tc, raw_ty, p_name, stripped
                    ));
                }
                untrusted = true;
                String::from(stripped)
            } else {
                raw_ty
            };
            if let Some(SurfaceTok::AtIdent(tag)) = self.peek().cloned() {
                let st = self.next_spanned()?;
                match tag.to_ascii_lowercase().as_str() {
                    "untrusted" => untrusted = true,
                    "safe" => untrusted = false,
                    other => {
                        self.strict_mode = prev_strict;
                        return Err(format!(
                            "error[BAD_TYPE] line {}, col {}: Unknown taint annotation '@{}'",
                            st.line, st.col, other
                        ));
                    }
                }
            }
            params.push(SurfaceParam {
                name: p_name,
                ty: clean_ty,
                untrusted,
                line: tl,
                col: tc,
            });
            if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                let _ = self.next_spanned()?;
            }
        }
        self.expect_sym(")")?;
        cap_mask |= self.consume_at_annotations()?;
        self.expect_sym("->")?;
        let (ret_ty, rl, rc) = self.expect_ident_spanned()?;
        cap_mask |= self.consume_at_annotations()?;
        let body = self.parse_block()?;
        cap_mask |= self.extra_fn_caps;
        self.strict_mode = prev_strict;

        Ok(SurfaceFnDef {
            fn_id_attr,
            cap_mask,
            step_budget_attr,
            mem_budget_attr,
            budget_span,
            strict_attr: fn_strict,
            is_route: false,
            route_method: None,
            route_path: None,
            name: fn_name,
            fn_line: fn_kw_line,
            fn_col: fn_kw_col,
            params,
            ret_ty,
            ret_line: rl,
            ret_col: rc,
            body,
        })
    }

    pub(crate) fn parse_block(&mut self) -> Result<Vec<SurfaceStmt>, String> {
        self.expect_sym("{")?;
        let mut stmts = Vec::new();
        while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("}"))) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect_sym("}")?;
        Ok(stmts)
    }

    pub(crate) fn parse_stmt(&mut self) -> Result<SurfaceStmt, String> {
        let _ = self.consume_at_annotations()?;
        let (line, col) = self.current_span();
        match self.peek().cloned() {
            Some(SurfaceTok::Ident(ref kw)) if kw == "let" => {
                let _ = self.next_spanned()?;
                let (name, nl, nc) = self.expect_ident_spanned()?;
                let mut ty_opt: Option<(String, usize, usize)> = None;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(":"))) {
                    let _ = self.next_spanned()?;
                    let (ty_ann, tl, tc) = self.expect_ident_spanned()?;
                    let _ = map_omni_surface_type_inner(&ty_ann, tl, tc, self.strict_mode)?;
                    let _ = self.consume_at_annotations()?;
                    ty_opt = Some((ty_ann, tl, tc));
                } else if self.strict_mode {
                    return Err(format!(
                        "error[BAD_TYPE] line {}, col {}: Missing explicit type annotation on 'let {}' (strict mode requires 'let {}: <ty> = ...;')",
                        nl, nc, name, name
                    ));
                }
                self.expect_sym("=")?;
                let expr = self.parse_expr()?;
                let _ = self.consume_at_annotations()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Let(name, ty_opt, expr, nl, nc))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "return" || kw == "respond" || kw == "respond_json" || kw == "respond_html" => {
                if kw == "respond" || kw == "respond_json" || kw == "respond_html" {
                    let expr = self.parse_expr()?;
                    let _ = self.consume_at_annotations()?;
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                        let _ = self.next_spanned()?;
                    }
                    return Ok(SurfaceStmt::Ret(expr, line, col));
                }
                let _ = self.next_spanned()?;
                let expr = self.parse_expr()?;
                let _ = self.consume_at_annotations()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Ret(expr, line, col))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "do_while" => {
                let _ = self.next_spanned()?;
                let body = self.parse_block()?;
                let cond = self.parse_expr()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::DoWhile(body, cond, line, col))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "while" => {
                let _ = self.next_spanned()?;
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::While(cond, body, line, col))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "label" => {
                let _ = self.next_spanned()?;
                let (lid, ll, lc) = self.expect_int_spanned()?;
                if !(0..=15).contains(&lid) {
                    return Err(format!(
                        "error[UNRESOLVED_LABEL] line {}, col {}: Label id {} out of range 0..=15",
                        ll, lc, lid
                    ));
                }
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Label(lid as u16, ll, lc))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "goto" => {
                let _ = self.next_spanned()?;
                let (lid, ll, lc) = self.expect_int_spanned()?;
                if !(0..=15).contains(&lid) {
                    return Err(format!(
                        "error[UNRESOLVED_LABEL] line {}, col {}: Goto label id {} out of range 0..=15",
                        ll, lc, lid
                    ));
                }
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Goto(lid as u16, ll, lc))
            }
            Some(SurfaceTok::Ident(_))
                if matches!(
                    self.tokens.get(self.pos + 1).map(|s| &s.tok),
                    Some(SurfaceTok::Sym(op)) if op == "=" || op == "+=" || op == "-=" || op == "*="
                ) =>
            {
                let (name, nl, nc) = self.expect_ident_spanned()?;
                let assign_tok = self.next_spanned()?;
                let assign_op = match assign_tok.tok {
                    SurfaceTok::Sym(s) => s,
                    _ => String::from("="),
                };
                let rhs = self.parse_expr()?;
                let _ = self.consume_at_annotations()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                let expr = match assign_op.as_str() {
                    "+=" => SurfaceExpr::BinOp(
                        String::from("IADD"),
                        Box::new(SurfaceExpr::Var(name.clone(), nl, nc)),
                        Box::new(rhs),
                        nl,
                        nc,
                    ),
                    "-=" => SurfaceExpr::BinOp(
                        String::from("ISUB"),
                        Box::new(SurfaceExpr::Var(name.clone(), nl, nc)),
                        Box::new(rhs),
                        nl,
                        nc,
                    ),
                    "*=" => SurfaceExpr::BinOp(
                        String::from("IMUL"),
                        Box::new(SurfaceExpr::Var(name.clone(), nl, nc)),
                        Box::new(rhs),
                        nl,
                        nc,
                    ),
                    _ => rhs,
                };
                Ok(SurfaceStmt::Assign(name, expr, nl, nc))
            }
            _ => {
                let expr = self.parse_expr()?;
                let _ = self.consume_at_annotations()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from(";"))) {
                    let _ = self.next_spanned()?;
                }
                Ok(SurfaceStmt::Expr(expr, line, col))
            }
        }
    }

    pub(crate) fn parse_expr(&mut self) -> Result<SurfaceExpr, String> {
        let e = self.parse_cmp()?;
        let _ = self.consume_at_annotations()?;
        Ok(e)
    }

    pub(crate) fn parse_cmp(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitor()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            match op.as_str() {
                "==" | "!=" | "<" | ">" | "<=>" => {
                    let mnem = match op.as_str() {
                        "==" => "CEQ",
                        "!=" => "CNE",
                        "<" => "CLT",
                        ">" => "CGT",
                        "<=>" => "TCMP",
                        _ => unreachable!(),
                    };
                    let st = self.next_spanned()?;
                    let rhs = self.parse_bitor()?;
                    lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
                }
                "<=" => {
                    let st = self.next_spanned()?;
                    let rhs = self.parse_bitor()?;
                    let gt = SurfaceExpr::BinOp(String::from("CGT"), Box::new(lhs), Box::new(rhs), st.line, st.col);
                    lhs = SurfaceExpr::UnOp(String::from("TNOT"), Box::new(gt), st.line, st.col);
                }
                ">=" => {
                    let st = self.next_spanned()?;
                    let rhs = self.parse_bitor()?;
                    let lt = SurfaceExpr::BinOp(String::from("CLT"), Box::new(lhs), Box::new(rhs), st.line, st.col);
                    lhs = SurfaceExpr::UnOp(String::from("TNOT"), Box::new(lt), st.line, st.col);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    pub(crate) fn parse_bitor(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitxor()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("|"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_bitxor()?;
            lhs = SurfaceExpr::BinOp(String::from("BOR"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    pub(crate) fn parse_bitxor(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitand()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("^"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_bitand()?;
            lhs = SurfaceExpr::BinOp(String::from("BXOR"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    pub(crate) fn parse_bitand(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_shift()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("&"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_shift()?;
            lhs = SurfaceExpr::BinOp(String::from("BAND"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    pub(crate) fn parse_shift(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_add()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            let mnem = match op.as_str() {
                "<<" => "SHL",
                ">>" => "SHR",
                _ => break,
            };
            let st = self.next_spanned()?;
            let rhs = self.parse_add()?;
            lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    pub(crate) fn parse_add(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_mul()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            let mnem = match op.as_str() {
                "+" => "IADD",
                "-" => "ISUB",
                _ => break,
            };
            let st = self.next_spanned()?;
            let rhs = self.parse_mul()?;
            lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    pub(crate) fn parse_mul(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_unary()?;
        while let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            let mnem = match op.as_str() {
                "*" => "IMUL",
                "/" => "IDIV",
                "%" => "IMOD",
                _ => break,
            };
            let st = self.next_spanned()?;
            let rhs = self.parse_unary()?;
            lhs = SurfaceExpr::BinOp(String::from(mnem), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    pub(crate) fn parse_unary(&mut self) -> Result<SurfaceExpr, String> {
        if let Some(SurfaceTok::Op(op)) = self.peek().cloned() {
            if op == "-" {
                let st = self.next_spanned()?;
                let sub = self.parse_unary()?;
                if let SurfaceExpr::Int(v, _, _) = sub {
                    return Ok(SurfaceExpr::Int(v.wrapping_neg(), st.line, st.col));
                }
                return Ok(SurfaceExpr::UnOp(String::from("INEG"), Box::new(sub), st.line, st.col));
            }
            if op == "+" {
                let _ = self.next_spanned()?;
                return self.parse_unary();
            }
            if op == "~" {
                let st = self.next_spanned()?;
                let sub = self.parse_unary()?;
                return Ok(SurfaceExpr::UnOp(String::from("BNOT"), Box::new(sub), st.line, st.col));
            }
            if op == "!" {
                let st = self.next_spanned()?;
                let sub = self.parse_unary()?;
                return Ok(SurfaceExpr::UnOp(String::from("TNOT"), Box::new(sub), st.line, st.col));
            }
        }
        self.parse_primary()
    }

    pub(crate) fn parse_primary(&mut self) -> Result<SurfaceExpr, String> {
        let _ = self.consume_at_annotations()?;
        let (line, col) = self.current_span();
        match self.peek().cloned() {
            Some(SurfaceTok::Int(v)) => {
                let st = self.next_spanned()?;
                Ok(SurfaceExpr::Int(v, st.line, st.col))
            }
            Some(SurfaceTok::Trit(s, tv)) => {
                let st = self.next_spanned()?;
                Ok(SurfaceExpr::Trit(s, tv, st.line, st.col))
            }
            Some(SurfaceTok::StrLit(s)) => {
                let st = self.next_spanned()?;
                Ok(SurfaceExpr::StrLit(s, st.line, st.col))
            }
            Some(SurfaceTok::Sym(ref s)) if s == "[" => {
                let st = self.next_spanned()?;
                let mut trits = Vec::new();
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("]"))) {
                    let elem = self.parse_expr()?;
                    match elem {
                        SurfaceExpr::Trit(_, tv, _, _) => trits.push(tv),
                        SurfaceExpr::Int(v, el, ec) if (-1..=1).contains(&v) => {
                            if self.strict_mode {
                                return Err(format!(
                                    "error[AMBIGUOUS_GRAMMAR] line {}, col {}: Trit vector literal elements must use canonical trit tokens (-1t, 0t, +1t) in strict mode",
                                    el, ec
                                ));
                            }
                            trits.push(v as i8);
                        }
                        other => {
                            let (el, ec) = other.span();
                            return Err(format!(
                                "error[BAD_TYPE] line {}, col {}: Trit vector literal elements must be balanced trits (-1t, 0t, +1t)",
                                el, ec
                            ));
                        }
                    }
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                }
                self.expect_sym("]")?;
                if trits.is_empty() {
                    return Err(format!(
                        "error[BAD_TYPE] line {}, col {}: Trit vector literal cannot be empty",
                        st.line, st.col
                    ));
                }
                Ok(SurfaceExpr::TritVecLit(trits, st.line, st.col))
            }
            Some(SurfaceTok::Sym(ref s)) if s == "(" => {
                let _ = self.next_spanned()?;
                let e = self.parse_expr()?;
                self.expect_sym(")")?;
                Ok(e)
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "if_pos" || kw == "if_neg" => {
                let br_mnem = if kw == "if_pos" { "BR_POS" } else { "BR_NEG" };
                let st = self.next_spanned()?;
                let cond = self.parse_expr()?;
                let then_blk = self.parse_block()?;
                self.expect_ident_kw("else")?;
                let else_blk = self.parse_block()?;
                Ok(SurfaceExpr::IfBranch(
                    String::from(br_mnem),
                    Box::new(cond),
                    then_blk,
                    else_blk,
                    st.line,
                    st.col,
                ))
            }
            Some(SurfaceTok::Ident(ref kw)) if kw == "match_trit" => {
                let st = self.next_spanned()?;
                let cond = self.parse_expr()?;
                self.expect_sym("{")?;
                let mut neg_arm: Option<Vec<SurfaceStmt>> = None;
                let mut zero_arm: Option<Vec<SurfaceStmt>> = None;
                let mut pos_arm: Option<Vec<SurfaceStmt>> = None;
                while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("}"))) {
                    let (arm_name, al, ac) = self.expect_ident_spanned()?;
                    self.expect_sym("=>")?;
                    let arm_stmts = if self.peek() == Some(&SurfaceTok::Sym(String::from("{"))) {
                        self.parse_block()?
                    } else {
                        let e = self.parse_expr()?;
                        let (el, ec) = e.span();
                        vec![SurfaceStmt::Expr(e, el, ec)]
                    };
                    if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                        let _ = self.next_spanned()?;
                    }
                    match arm_name.as_str() {
                        "neg" => neg_arm = Some(arm_stmts),
                        "zero" => zero_arm = Some(arm_stmts),
                        "pos" => pos_arm = Some(arm_stmts),
                        other => {
                            return Err(format!(
                                "error[SYNTAX_ERROR] line {}, col {}: Unknown match_trit arm '{}' (expected neg, zero, pos)",
                                al, ac, other
                            ));
                        }
                    }
                }
                self.expect_sym("}")?;
                let neg = neg_arm.ok_or_else(|| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: match_trit missing 'neg' arm",
                        st.line, st.col
                    )
                })?;
                let zero = zero_arm.ok_or_else(|| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: match_trit missing 'zero' arm",
                        st.line, st.col
                    )
                })?;
                let pos = pos_arm.ok_or_else(|| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: match_trit missing 'pos' arm",
                        st.line, st.col
                    )
                })?;
                Ok(SurfaceExpr::MatchTrit(Box::new(cond), neg, zero, pos, st.line, st.col))
            }
            Some(SurfaceTok::Ident(ident)) => {
                let st = self.next_spanned()?;
                if self.peek() == Some(&SurfaceTok::Sym(String::from("."))) {
                    let _ = self.next_spanned()?;
                    let (method_name, _ml, _mc) = self.expect_ident_spanned()?;
                    self.expect_sym("(")?;
                    let mut args = vec![SurfaceExpr::Var(ident.clone(), st.line, st.col)];
                    while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                        args.push(self.parse_expr()?);
                        if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                            let _ = self.next_spanned()?;
                        }
                    }
                    self.expect_sym(")")?;
                    let _ = self.consume_at_annotations()?;
                    let call_name = if ident == "req" && method_name == "param" {
                        String::from("req_param")
                    } else if (ident == "db" || ident == "kv") && method_name == "query" {
                        String::from("db_query")
                    } else if (ident == "db" || ident == "kv") && method_name == "get" {
                        String::from("db_get")
                    } else if (ident == "db" || ident == "kv") && method_name == "put" {
                        String::from("kv_put")
                    } else {
                        format!("{}_{}", ident, method_name)
                    };
                    if call_name == "db_query" || call_name == "db_get" || call_name == "kv_put" {
                        args.remove(0);
                    }
                    return Ok(SurfaceExpr::Call(call_name, args, st.line, st.col));
                }
                if self.peek() == Some(&SurfaceTok::Sym(String::from("("))) {
                    let _ = self.next_spanned()?;
                    let mut args = Vec::new();
                    while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from(")"))) {
                        args.push(self.parse_expr()?);
                        if self.peek() == Some(&SurfaceTok::Sym(String::from(","))) {
                            let _ = self.next_spanned()?;
                        }
                    }
                    self.expect_sym(")")?;
                    let _ = self.consume_at_annotations()?;
                    if let Some((m_params, m_body)) = self.macros.get(&ident).cloned() {
                        if args.len() != m_params.len() {
                            return Err(format!(
                                "error[ARITY_MISMATCH] line {}, col {}: Hygienic macro '{}' expects {} arguments, got {}",
                                st.line, st.col, ident, m_params.len(), args.len()
                            ));
                        }
                        let mut param_map = BTreeMap::new();
                        for (p_name, arg_expr) in m_params.into_iter().zip(args.into_iter()) {
                            param_map.insert(p_name, arg_expr);
                        }
                        return substitute_macro_expr(&m_body, &param_map, st.line, st.col);
                    }
                    Ok(SurfaceExpr::Call(ident, args, st.line, st.col))
                } else {
                    Ok(SurfaceExpr::Var(ident, st.line, st.col))
                }
            }
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Unexpected token in surface expression: {:?}",
                line, col, other
            )),
        }
    }
}

pub(crate) fn map_omni_surface_type_inner(ty: &str, line: usize, col: usize, strict: bool) -> Result<&'static str, String> {
    let low = ty.to_ascii_lowercase();
    if strict && matches!(low.as_str(), "int" | "byte_buf" | "u16_array" | "tvec") {
        return Err(format!(
            "error[AMBIGUOUS_GRAMMAR] line {}, col {}: Non-canonical type alias '{}'; use 'i64', 'trit', 'bytes', 'array', 'trit_vec', or 'str' in strict mode",
            line, col, ty
        ));
    }
    match low.as_str() {
        "i64" | "int" => Ok("I64"),
        "trit" => Ok("TRIT"),
        "bytes" | "byte_buf" | "pack5" => Ok("BYTES"),
        "array" | "u16_array" => Ok("ARRAY"),
        "trit_vec" | "tvec" => Ok("TRIT_VEC"),
        "str" => Ok("STR"),
        "uuid" => Ok("UUID"),
        "arena" => Ok("ARENA"),
        "dom" => Ok("DOM"),
        "struct" => Ok("STRUCT"),
        _ => Err(format!(
            "error[BAD_TYPE] line {}, col {}: Unknown surface type '{}'",
            line, col, ty
        )),
    }
}

pub(crate) fn fnv1a_32_positive(s: &str) -> i64 {
    let mut h: u32 = 0x811c9dc5;
    for &b in s.as_bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    ((h & 0x3FFF_FFFF) as i64).max(1)
}

pub(crate) struct SurfaceLowerCtx {
    pub(crate) var_map: BTreeMap<String, String>,
    pub(crate) tainted_operands: BTreeSet<String>,
    pub(crate) fn_sigs: BTreeMap<String, (u16, usize, String)>,
    pub(crate) next_reg: usize,
    pub(crate) next_lbl: u16,
    pub(crate) defined_labels: Vec<u16>,
    pub(crate) used_labels: Vec<(u16, usize, usize)>,
    pub(crate) ret_ty_tok: u16,
    pub(crate) emit_asm: bool,
    pub(crate) cur_span: (usize, usize),
    pub(crate) lines: Vec<String>,
    pub(crate) packets: Vec<Vec<u16>>,
    pub(crate) pkt_spans: Vec<(usize, usize)>,
}

impl SurfaceLowerCtx {
    pub(crate) fn new(
        params: &[SurfaceParam],
        fn_sigs: BTreeMap<String, (u16, usize, String)>,
        ret_ty_tok: u16,
        emit_asm: bool,
    ) -> Self {
        let mut var_map = BTreeMap::new();
        let mut tainted_operands = BTreeSet::new();
        for (idx, p) in params.iter().enumerate() {
            let op_str = format!("${}", idx);
            if p.untrusted {
                tainted_operands.insert(op_str.clone());
            }
            var_map.insert(p.name.clone(), op_str);
        }
        Self {
            var_map,
            tainted_operands,
            fn_sigs,
            next_reg: 0,
            next_lbl: 1,
            defined_labels: Vec::new(),
            used_labels: Vec::new(),
            ret_ty_tok,
            emit_asm,
            cur_span: (1, 1),
            lines: Vec::new(),
            packets: Vec::new(),
            pkt_spans: Vec::new(),
        }
    }

    pub(crate) fn emit(&mut self, mut line: String) -> Result<(), String> {
        let parts: Vec<String> = line.split_whitespace().map(String::from).collect();
        if parts.len() >= 4 && parts[1] == "=" {
            let dst = &parts[0];
            let mnem = &parts[2];
            let base_mnem = mnem
                .strip_suffix("@UNTRUSTED")
                .or_else(|| mnem.strip_suffix("@SAFE"))
                .unwrap_or(mnem.as_str());
            let is_sanitizer = matches!(base_mnem, "SAN_SQL" | "SAN_DOM" | "SAN_PRM" | "AUTH_VER");
            if is_sanitizer {
                self.tainted_operands.remove(dst);
            } else {
                let any_arg_tainted = mnem.ends_with("@UNTRUSTED")
                    || parts[3..].iter().any(|op| self.tainted_operands.contains(op));
                if any_arg_tainted {
                    self.tainted_operands.insert(dst.clone());
                    if !mnem.ends_with("@UNTRUSTED") && !mnem.ends_with("@SAFE") {
                        let mut rebuilt = vec![parts[0].clone(), parts[1].clone(), format!("{}@UNTRUSTED", parts[2])];
                        rebuilt.extend_from_slice(&parts[3..]);
                        line = rebuilt.join(" ");
                    }
                }
            }
        }
        if self.emit_asm {
            self.lines.push(line);
        } else {
            let pkt = encode_asm_ins_packet_u16(&line, self.ret_ty_tok)?;
            self.packets.push(pkt);
            self.pkt_spans.push((self.cur_span.0.max(1), self.cur_span.1.max(1)));
        }
        Ok(())
    }

    pub(crate) fn alloc_reg(&mut self, line: usize, col: usize) -> Result<String, String> {
        if self.next_reg >= 192 {
            return Err(format!(
                "error[BUDGET_EXCEEDED] line {}, col {}: Surface function exceeded 192 SSA registers",
                line, col
            ));
        }
        let r = format!("%{}", self.next_reg);
        self.next_reg += 1;
        Ok(r)
    }

    pub(crate) fn alloc_lbl(&mut self, line: usize, col: usize) -> Result<u16, String> {
        if self.next_lbl > 15 {
            return Err(format!(
                "error[BUDGET_EXCEEDED] line {}, col {}: Surface function exceeded 15 basic-block labels",
                line, col
            ));
        }
        let l = self.next_lbl;
        self.next_lbl += 1;
        Ok(l)
    }

    pub(crate) fn eval_to_operand(&mut self, expr: &SurfaceExpr) -> Result<String, String> {
        match expr {
            SurfaceExpr::Int(v, _, _) => Ok(v.to_string()),
            SurfaceExpr::Trit(s, _, _, _) => Ok(s.clone()),
            SurfaceExpr::Var(v, line, col) => self
                .var_map
                .get(v)
                .cloned()
                .ok_or_else(|| format!("error[UNDEF_VAR] line {}, col {}: Undefined variable '{}'", line, col, v)),
            _ => self.lower_expr_into(expr, None, false),
        }
    }

    pub(crate) fn eval_host_arg_to_operand(&mut self, expr: &SurfaceExpr) -> Result<String, String> {
        match expr {
            SurfaceExpr::StrLit(s, _, _) => {
                if let Ok(n) = s.trim().parse::<i64>() {
                    Ok(n.to_string())
                } else {
                    Ok(fnv1a_32_positive(s).to_string())
                }
            }
            other => self.eval_to_operand(other),
        }
    }

    pub(crate) fn lower_trit_vec_pack5(
        &mut self,
        trits: &[i8],
        target_reg: Option<&str>,
        unpack_to_tvec: bool,
        line: usize,
        col: usize,
    ) -> Result<String, String> {
        let packed_bytes = pack5_encode(trits);
        let r_bytes = if !unpack_to_tvec {
            match target_reg {
                Some(r) => String::from(r),
                None => self.alloc_reg(line, col)?,
            }
        } else {
            self.alloc_reg(line, col)?
        };
        self.emit(format!("{} = ANEW 0", r_bytes))?;
        for chunk in packed_bytes.chunks(24) {
            let byte_strs: Vec<String> = chunk.iter().map(|b| b.to_string()).collect();
            self.emit(format!("{} = ISET {} {}", r_bytes, r_bytes, byte_strs.join(" ")))?;
        }
        if !unpack_to_tvec {
            return Ok(r_bytes);
        }
        let dst = match target_reg {
            Some(r) => String::from(r),
            None => self.alloc_reg(line, col)?,
        };
        self.emit(format!("{} = UNPACK5 {} {}", dst, r_bytes, trits.len()))?;
        Ok(dst)
    }

    pub(crate) fn lower_call_rhs(&mut self, name: &str, args: &[SurfaceExpr], line: usize, col: usize) -> Result<String, String> {
        // 1. Check user-defined functions in the module / included files first
        if let Some((callee_id, expected_arity, callee_ret_ty)) = self.fn_sigs.get(name).cloned() {
            if args.len() != expected_arity {
                return Err(format!(
                    "error[ARITY_MISMATCH] line {}, col {}: Function '{}' expects {} arguments, got {}",
                    line, col, name, expected_arity, args.len()
                ));
            }
            let mut op_args = Vec::with_capacity(args.len() + 1);
            op_args.push(format!("@fn{}", callee_id));
            for a in args {
                op_args.push(self.eval_to_operand(a)?);
            }
            let call_mnem = match callee_ret_ty.as_str() {
                "TRIT" => "CALL_TRIT",
                "BYTES" => "CALL_BYTES",
                "TRIT_VEC" => "CALL_TRIT_VEC",
                "STR" => "CALL_STR",
                _ => "CALL_I64",
            };
            return Ok(format!("{} {}", call_mnem, op_args.join(" ")));
        }

        let low = name.to_ascii_lowercase();

        // 2. Host ABI opcodes (0x7F64..=0x7F70)
        match low.as_str() {
            "req_param" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'req_param' expects 1 or 2 arguments, got {}",
                        line, col, args.len()
                    ));
                }
                let req_op = if args.len() == 2 {
                    self.eval_host_arg_to_operand(&args[0])?
                } else if self.var_map.contains_key("req") {
                    self.var_map.get("req").cloned().unwrap()
                } else {
                    self.eval_host_arg_to_operand(&args[0])?
                };
                let key_op = if args.len() == 2 {
                    self.eval_host_arg_to_operand(&args[1])?
                } else {
                    String::from("0")
                };
                return Ok(format!("REQ_PAR@SAFE {} {}", req_op, key_op));
            }
            "db_get" | "kv_get" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic '{}' expects 1 or 2 arguments, got {}",
                        line, col, name, args.len()
                    ));
                }
                let key_op = self.eval_host_arg_to_operand(args.last().unwrap())?;
                return Ok(format!("DB_GET {}", key_op));
            }
            "db_query" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'db_query' expects 1 or 2 arguments, got {}",
                        line, col, args.len()
                    ));
                }
                let q_op = self.eval_host_arg_to_operand(&args[0])?;
                let p_op = if args.len() == 2 {
                    self.eval_host_arg_to_operand(&args[1])?
                } else {
                    String::from("0")
                };
                return Ok(format!("DB_QRY {} {}", q_op, p_op));
            }
            "db_count" | "kv_count" => {
                let c_op = if let Some(first) = args.first() {
                    self.eval_host_arg_to_operand(first)?
                } else {
                    String::from("0")
                };
                return Ok(format!("DB_CNT {}", c_op));
            }
            "kv_put" | "db_put" => {
                if args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic '{}' expects 2 arguments (key, value), got {}",
                        line, col, name, args.len()
                    ));
                }
                let k_op = self.eval_host_arg_to_operand(&args[0])?;
                let v_op = self.eval_host_arg_to_operand(&args[1])?;
                return Ok(format!("KV_PUT {} {}", k_op, v_op));
            }
            "respond" | "respond_json" | "respond_html" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic '{}' expects 1 or 2 arguments (status, body), got {}",
                        line, col, name, args.len()
                    ));
                }
                let mnem = if low == "respond_html" { "RSP_HTML" } else { "RSP_JSON" };
                if args.len() == 2 {
                    let st_op = self.eval_host_arg_to_operand(&args[0])?;
                    let body_op = self.eval_host_arg_to_operand(&args[1])?;
                    return Ok(format!("{} {} {}", mnem, st_op, body_op));
                } else {
                    let body_op = self.eval_host_arg_to_operand(&args[0])?;
                    return Ok(format!("{} 200 {}", mnem, body_op));
                }
            }
            "auth_verify" => {
                if args.len() != 1 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'auth_verify' expects 1 argument, got {}",
                        line, col, args.len()
                    ));
                }
                let tok_op = self.eval_host_arg_to_operand(&args[0])?;
                return Ok(format!("AUTH_VER {}", tok_op));
            }
            _ => {}
        }

        let mut op_args = Vec::with_capacity(args.len());
        for a in args {
            op_args.push(self.eval_to_operand(a)?);
        }
        let unary_mnem = match low.as_str() {
            "iabs" | "abs" => Some("IABS"),
            "ineg" => Some("INEG"),
            "bnot" => Some("BNOT"),
            "tnot" => Some("TNOT"),
            "anew" => Some("ANEW"),
            "slen" | "pack5_len" => Some("SLEN"),
            "cint" | "mov" => Some("CINT"),
            "ctrit" => Some("CTRIT"),
            "pack5" | "tpack5" => Some("PACK5"),
            "egate" | "decision_hook" | "pack5_gate" => Some("EGATE"),
            "tlogic" | "decide_trit" => Some("TLOGIC"),
            "lookahead" => Some("LOOKAHEAD"),
            "san_sql" | "sanitize_sql" => Some("SAN_SQL"),
            "san_dom" | "sanitize_dom" => Some("SAN_DOM"),
            "san_prm" | "sanitize_prompt" => Some("SAN_PRM"),
            "fnv1a" | "hash_fnv1a" => Some("FNV1A"),
            "arena" => Some("ARENA"),
            "borrow_r" => Some("BORROW_R"),
            "borrow_w" => Some("BORROW_W"),
            "move_val" => Some("MOVE"),
            "s2i" | "str_to_int" => Some("S2I"),
            "i2s" | "int_to_str" => Some("I2S"),
            _ => None,
        };
        if let Some(m) = unary_mnem {
            if op_args.len() != 1 {
                return Err(format!(
                    "error[ARITY_MISMATCH] line {}, col {}: Intrinsic '{}' expects 1 argument, got {}",
                    line, col, name, op_args.len()
                ));
            }
            return Ok(format!("{} {}", m, op_args[0]));
        }
        let binary_mnem = match low.as_str() {
            "iadd" => Some("IADD"),
            "isub" => Some("ISUB"),
            "imul" => Some("IMUL"),
            "idiv" => Some("IDIV"),
            "imod" => Some("IMOD"),
            "imin" | "min" => Some("IMIN"),
            "imax" | "max" => Some("IMAX"),
            "band" => Some("BAND"),
            "bor" => Some("BOR"),
            "bxor" => Some("BXOR"),
            "shl" => Some("SHL"),
            "shr" => Some("SHR"),
            "ceq" => Some("CEQ"),
            "cne" => Some("CNE"),
            "clt" => Some("CLT"),
            "cgt" => Some("CGT"),
            "tand" => Some("TAND"),
            "tor" => Some("TOR"),
            "txor" => Some("TXOR"),
            "tcmp" => Some("TCMP"),
            "iget" | "pack5_get" => Some("IGET"),
            "unpack5" | "tunpack5" => Some("UNPACK5"),
            "tdot" | "pack5_tdot" => Some("TDOT"),
            "scat" | "sconc" => Some("SCAT"),
            _ => None,
        };
        if let Some(m) = binary_mnem {
            if op_args.len() != 2 {
                return Err(format!(
                    "error[ARITY_MISMATCH] line {}, col {}: Intrinsic '{}' expects 2 arguments, got {}",
                    line, col, name, op_args.len()
                ));
            }
            return Ok(format!("{} {} {}", m, op_args[0], op_args[1]));
        }
        match low.as_str() {
            "tmaj" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'tmaj' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("TMAJ {} {} {}", op_args[0], op_args[1], op_args[2]))
            }
            "sel_trit" | "cmov" => {
                if op_args.len() != 4 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic '{}' expects 4 arguments, got {}",
                        line, col, name, op_args.len()
                    ));
                }
                let m = if low == "sel_trit" { "SEL_TRIT" } else { "CMOV" };
                Ok(format!("{} {} {} {} {}", m, op_args[0], op_args[1], op_args[2], op_args[3]))
            }
            "iset_i32" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'iset_i32' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -4 {}", op_args[0], op_args[1]))
            }
            "iset_i64" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'iset_i64' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -8 {}", op_args[0], op_args[1]))
            }
            "iset_patch_i64" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'iset_patch_i64' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -9 {} {}", op_args[0], op_args[1], op_args[2]))
            }
            "iset" => {
                if op_args.len() < 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'iset' expects at least 2 arguments",
                        line, col
                    ));
                }
                Ok(format!("ISET {}", op_args.join(" ")))
            }
            _ => Err(format!(
                "error[UNKNOWN_FN] line {}, col {}: Unknown surface intrinsic or function '{}'",
                line, col, name
            )),
        }
    }


    fn first_dest_name_in_range(&self, start: usize, end: usize) -> Option<String> {
        if self.emit_asm {
            for line in &self.lines[start..end] {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[1] == "=" {
                    return Some(String::from(parts[0]));
                }
            }
            None
        } else {
            for pkt in &self.packets[start..end] {
                if let Some(&tok) = pkt.first() {
                    if (REG_BASE..REG_BASE + 192).contains(&tok) {
                        return Some(format!("%{}", tok - REG_BASE));
                    }
                }
            }
            None
        }
    }

    /// If a value-returning branch would emit temps before writing the join dest,
    /// define that dest first so SSA numbering stays a single contiguous join slot.
    /// Programs whose first branch already writes the dest (existing goldens) are unchanged.
    fn ensure_contiguous_join_slot(
        &mut self,
        region_start: usize,
        dst: &str,
        line: usize,
        col: usize,
    ) -> Result<(), String> {
        let end = if self.emit_asm {
            self.lines.len()
        } else {
            self.packets.len()
        };
        if self.first_dest_name_in_range(region_start, end).as_deref() == Some(dst) {
            return Ok(());
        }
        let asm = format!("{} = CINT 0", dst);
        if self.emit_asm {
            self.lines.insert(region_start, asm);
        } else {
            let pkt = encode_asm_ins_packet_u16(&asm, self.ret_ty_tok)?;
            self.packets.insert(region_start, pkt);
            self.pkt_spans.insert(region_start, (line.max(1), col.max(1)));
        }
        Ok(())
    }

    pub(crate) fn lower_block_into_target(
        &mut self,
        stmts: &[SurfaceStmt],
        dest_reg: &str,
        has_shared_join: bool,
        line: usize,
        col: usize,
    ) -> Result<(), String> {
        if stmts.is_empty() {
            return Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Empty branch block in control-flow expression",
                line, col
            ));
        }
        for s in &stmts[..stmts.len() - 1] {
            self.lower_stmt(s)?;
        }
        match &stmts[stmts.len() - 1] {
            SurfaceStmt::Expr(e, _, _) | SurfaceStmt::Ret(e, _, _) => {
                let _ = self.lower_expr_into(e, Some(dest_reg), has_shared_join)?;
                Ok(())
            }
            other => self.lower_stmt(other),
        }
    }

    pub(crate) fn lower_expr_into(
        &mut self,
        expr: &SurfaceExpr,
        target_reg: Option<&str>,
        has_shared_join: bool,
    ) -> Result<String, String> {
        self.cur_span = expr.span();
        match expr {
            SurfaceExpr::Int(v, line, col) => {
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.cur_span = (*line, *col);
                self.emit(format!("{} = CINT {}", dst, v))?;
                Ok(dst)
            }
            SurfaceExpr::Trit(s, _, line, col) => {
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.cur_span = (*line, *col);
                self.emit(format!("{} = CTRIT {}", dst, s))?;
                Ok(dst)
            }
            SurfaceExpr::TritVecLit(trits, line, col) => {
                self.cur_span = (*line, *col);
                self.lower_trit_vec_pack5(trits, target_reg, true, *line, *col)
            }
            SurfaceExpr::StrLit(s, line, col) => {
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                let mut hex_str = String::with_capacity(s.len() * 2);
                for &b in s.as_bytes() {
                    hex_str.push_str(&format!("{:02x}", b));
                }
                self.cur_span = (*line, *col);
                self.emit(format!("{} = CSTR@UNTRUSTED @str:{}", dst, hex_str))?;
                Ok(dst)
            }
            SurfaceExpr::Var(v, line, col) => {
                let src = self
                    .var_map
                    .get(v)
                    .cloned()
                    .ok_or_else(|| format!("error[UNDEF_VAR] line {}, col {}: Undefined variable '{}'", line, col, v))?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.cur_span = (*line, *col);
                self.emit(format!("{} = CINT {}", dst, src))?;
                Ok(dst)
            }
            SurfaceExpr::UnOp(mnem, sub, line, col) => {
                let op_s = self.eval_to_operand(sub)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.cur_span = (*line, *col);
                self.emit(format!("{} = {} {}", dst, mnem, op_s))?;
                Ok(dst)
            }
            SurfaceExpr::BinOp(mnem, lhs, rhs, line, col) => {
                let l_op = self.eval_to_operand(lhs)?;
                let r_op = self.eval_to_operand(rhs)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.cur_span = (*line, *col);
                self.emit(format!("{} = {} {} {}", dst, mnem, l_op, r_op))?;
                Ok(dst)
            }
            SurfaceExpr::Call(name, args, line, col) => {
                if name.eq_ignore_ascii_case("pack5_bytes") {
                    if args.len() != 1 {
                        return Err(format!(
                            "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'pack5_bytes' expects 1 trit-vector literal argument",
                            line, col
                        ));
                    }
                    if let SurfaceExpr::TritVecLit(ref trits, l, c) = args[0] {
                        self.cur_span = (l, c);
                        return self.lower_trit_vec_pack5(trits, target_reg, false, l, c);
                    } else {
                        return Err(format!(
                            "error[BAD_TYPE] line {}, col {}: Intrinsic 'pack5_bytes' requires a '[+1t, -1t, ...]' literal",
                            line, col
                        ));
                    }
                }
                let rhs = self.lower_call_rhs(name, args, *line, *col)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                self.cur_span = (*line, *col);
                self.emit(format!("{} = {}", dst, rhs))?;
                Ok(dst)
            }
            SurfaceExpr::IfBranch(br_mnem, cond_expr, then_blk, else_blk, line, col) => {
                let cond_op = self.eval_to_operand(cond_expr)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                let l_then = self.alloc_lbl(*line, *col)?;
                self.cur_span = (*line, *col);
                self.emit(format!("{} {} {}", br_mnem, cond_op, l_then))?;
                let saved_vars = self.var_map.clone();
                let else_start = if self.emit_asm { self.lines.len() } else { self.packets.len() };
                self.lower_block_into_target(else_blk, &dst, false, *line, *col)?;
                self.ensure_contiguous_join_slot(else_start, &dst, *line, *col)?;
                self.var_map = saved_vars.clone();
                let jmp_placeholder_idx = if self.emit_asm { self.lines.len() } else { self.packets.len() };
                if self.emit_asm {
                    self.lines.push(String::from("JMP __JOIN__"));
                } else {
                    self.packets.push(vec![0x7F41, 0xFFFF, TOK_INS_END]);
                    self.pkt_spans.push(((*line).max(1), (*col).max(1)));
                }
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_then))?;
                self.lower_block_into_target(then_blk, &dst, true, *line, *col)?;
                self.var_map = saved_vars;
                if !has_shared_join {
                    let l_join = self.alloc_lbl(*line, *col)?;
                    if self.emit_asm {
                        let join_str = l_join.to_string();
                        for l_str in &mut self.lines[jmp_placeholder_idx..] {
                            if l_str.contains("__JOIN__") {
                                *l_str = l_str.replace("__JOIN__", &join_str);
                            }
                        }
                    } else {
                        for pkt in &mut self.packets[jmp_placeholder_idx..] {
                            if pkt.len() >= 2 && pkt[0] == 0x7F41 && pkt[1] == 0xFFFF {
                                pkt[1] = LBL_BASE + l_join;
                            }
                        }
                    }
                    self.cur_span = (*line, *col);
                    self.emit(format!("LABEL {}", l_join))?;
                }
                Ok(dst)
            }
            SurfaceExpr::MatchTrit(cond_expr, neg_blk, zero_blk, pos_blk, line, col) => {
                let cond_op = self.eval_to_operand(cond_expr)?;
                let dst = match target_reg {
                    Some(r) => String::from(r),
                    None => self.alloc_reg(*line, *col)?,
                };
                let l_neg = self.alloc_lbl(*line, *col)?;
                let l_zero = self.alloc_lbl(*line, *col)?;
                let l_pos = self.alloc_lbl(*line, *col)?;
                let l_join = self.alloc_lbl(*line, *col)?;
                self.cur_span = (*line, *col);
                self.emit(format!("BR_TRIT {} {} {} {}", cond_op, l_neg, l_zero, l_pos))?;
                let saved_vars = self.var_map.clone();
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_neg))?;
                let neg_start = if self.emit_asm { self.lines.len() } else { self.packets.len() };
                self.lower_block_into_target(neg_blk, &dst, false, *line, *col)?;
                self.ensure_contiguous_join_slot(neg_start, &dst, *line, *col)?;
                self.var_map = saved_vars.clone();
                self.cur_span = (*line, *col);
                self.emit(format!("JMP {}", l_join))?;
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_zero))?;
                self.lower_block_into_target(zero_blk, &dst, false, *line, *col)?;
                self.var_map = saved_vars.clone();
                self.cur_span = (*line, *col);
                self.emit(format!("JMP {}", l_join))?;
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_pos))?;
                self.lower_block_into_target(pos_blk, &dst, false, *line, *col)?;
                self.var_map = saved_vars;
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_join))?;
                Ok(dst)
            }
        }
    }

    pub(crate) fn lower_stmt(&mut self, stmt: &SurfaceStmt) -> Result<(), String> {
        self.cur_span = stmt.span();
        match stmt {
            SurfaceStmt::Let(name, _ty_opt, expr, _line, _col) => {
                let reg = self.lower_expr_into(expr, None, false)?;
                self.var_map.insert(name.clone(), reg);
                Ok(())
            }
            SurfaceStmt::Assign(name, expr, line, col) => {
                let reg = self
                    .var_map
                    .get(name)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "error[UNDEF_VAR] line {}, col {}: Cannot reassign undeclared variable '{}'",
                            line, col, name
                        )
                    })?;
                if !reg.starts_with('%') {
                    return Err(format!(
                        "error[IMMUT_PARAM] line {}, col {}: Cannot reassign immutable function parameter '{}'",
                        line, col, name
                    ));
                }
                let _ = self.lower_expr_into(expr, Some(&reg), false)?;
                Ok(())
            }
            SurfaceStmt::DoWhile(body, cond_expr, line, col) => {
                if body.is_empty() {
                    return Err(format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Empty do_while loop body",
                        line, col
                    ));
                }
                let l_top = self.alloc_lbl(*line, *col)?;
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_top))?;
                for s in body {
                    self.lower_stmt(s)?;
                }
                let cond_op = self.eval_to_operand(cond_expr)?;
                self.cur_span = (*line, *col);
                self.emit(format!("BR_POS {} {}", cond_op, l_top))?;
                Ok(())
            }
            SurfaceStmt::While(cond_expr, body, line, col) => {
                if body.is_empty() {
                    return Err(format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Empty while loop body",
                        line, col
                    ));
                }
                let l_top = self.alloc_lbl(*line, *col)?;
                let l_body = self.alloc_lbl(*line, *col)?;
                let l_exit = self.alloc_lbl(*line, *col)?;
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", l_top))?;
                let cond_op = self.eval_to_operand(cond_expr)?;
                self.cur_span = (*line, *col);
                self.emit(format!("BR_POS {} {}", cond_op, l_body))?;
                self.emit(format!("JMP {}", l_exit))?;
                self.emit(format!("LABEL {}", l_body))?;
                for s in body {
                    self.lower_stmt(s)?;
                }
                self.cur_span = (*line, *col);
                self.emit(format!("JMP {}", l_top))?;
                self.emit(format!("LABEL {}", l_exit))?;
                Ok(())
            }
            SurfaceStmt::Label(lid, line, col) => {
                self.defined_labels.push(*lid);
                self.cur_span = (*line, *col);
                self.emit(format!("LABEL {}", lid))?;
                Ok(())
            }
            SurfaceStmt::Goto(lid, line, col) => {
                self.used_labels.push((*lid, *line, *col));
                self.cur_span = (*line, *col);
                self.emit(format!("JMP {}", lid))?;
                Ok(())
            }
            SurfaceStmt::Ret(expr, line, col) => {
                let op = self.eval_to_operand(expr)?;
                self.cur_span = (*line, *col);
                self.emit(format!("RET {}", op))?;
                Ok(())
            }
            SurfaceStmt::Expr(expr, line, col) => {
                if let SurfaceExpr::Call(cname, args, cl, cc) = expr {
                    let low = cname.to_ascii_lowercase();
                    if low == "br_pos" || low == "br_neg" {
                        if args.len() != 2 {
                            return Err(format!(
                                "error[ARITY_MISMATCH] line {}, col {}: '{}' expects 2 arguments",
                                cl, cc, cname
                            ));
                        }
                        let c_op = self.eval_to_operand(&args[0])?;
                        if let SurfaceExpr::Int(lid, ll, lc) = args[1] {
                            if !(0..=15).contains(&lid) {
                                return Err(format!(
                                    "error[UNRESOLVED_LABEL] line {}, col {}: Branch label {} out of range 0..=15",
                                    ll, lc, lid
                                ));
                            }
                            self.used_labels.push((lid as u16, ll, lc));
                            self.cur_span = (*cl, *cc);
                            self.emit(format!("{} {} {}", low.to_ascii_uppercase(), c_op, lid))?;
                            return Ok(());
                        } else {
                            return Err(format!(
                                "error[UNRESOLVED_LABEL] line {}, col {}: '{}' target must be an integer label id",
                                cl, cc, cname
                            ));
                        }
                    } else if low == "br_trit" {
                        if args.len() != 4 {
                            return Err(format!(
                                "error[ARITY_MISMATCH] line {}, col {}: 'br_trit' expects 4 arguments",
                                cl, cc
                            ));
                        }
                        let c_op = self.eval_to_operand(&args[0])?;
                        let mut lids = Vec::new();
                        for a in &args[1..] {
                            if let SurfaceExpr::Int(lid, ll, lc) = *a {
                                if !(0..=15).contains(&lid) {
                                    return Err(format!(
                                        "error[UNRESOLVED_LABEL] line {}, col {}: Branch label {} out of range 0..=15",
                                        ll, lc, lid
                                    ));
                                }
                                self.used_labels.push((lid as u16, ll, lc));
                                lids.push(lid.to_string());
                            } else {
                                return Err(format!(
                                    "error[UNRESOLVED_LABEL] line {}, col {}: 'br_trit' targets must be integer label ids",
                                    cl, cc
                                ));
                            }
                        }
                        self.cur_span = (*cl, *cc);
                        self.emit(format!("BR_TRIT {} {}", c_op, lids.join(" ")))?;
                        return Ok(());
                    } else if low == "drop" {
                        let mut ops = Vec::with_capacity(args.len());
                        for a in args {
                            ops.push(self.eval_to_operand(a)?);
                        }
                        self.cur_span = (*cl, *cc);
                        self.emit(format!("DROP {}", ops.join(" ")))?;
                        return Ok(());
                    } else if low == "respond" || low == "respond_json" || low == "respond_html" {
                        let op = self.lower_expr_into(expr, None, false)?;
                        self.cur_span = (*cl, *cc);
                        self.emit(format!("RET {}", op))?;
                        return Ok(());
                    }
                }
                let _ = self.lower_expr_into(expr, None, false)?;
                let _ = (line, col);
                Ok(())
            }
        }
    }

    pub(crate) fn check_unresolved_labels(&self) -> Result<(), String> {
        for &(lid, line, col) in &self.used_labels {
            if !self.defined_labels.contains(&lid) {
                return Err(format!(
                    "error[UNRESOLVED_LABEL] line {}, col {}: Unresolved branch target label {}",
                    line, col, lid
                ));
            }
        }
        Ok(())
    }
}

/// Looks up a built-in verified pure-Omni standard library module (`"std/prelude.omni"`, `"std/math.omni"`, `"std/trit.omni"`).
pub fn lookup_omni_stdlib_module(path: &str) -> Option<&'static str> {
    match path {
        "std/prelude.omni" => Some(
            "#[strict] fn std_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower: i64 = imax(x, lo); let clamped: i64 = imin(lower, hi); return clamped; }
#[strict] fn std_abs_diff(a: i64, b: i64) -> i64 { let d: i64 = a - b; let r: i64 = iabs(d); return r; }
#[strict] fn std_trit_mux(cond: trit, neg_v: i64, zero_v: i64, pos_v: i64) -> i64 { let r: i64 = sel_trit(cond, neg_v, zero_v, pos_v); return r; }
#[strict] fn std_consensus3(s0: trit, s1: trit, s2: trit) -> trit { let m: trit = tmaj(s0, s1, s2); return m; }",
        ),
        "std/math.omni" => Some(
            "#[strict] fn std_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower: i64 = imax(x, lo); let clamped: i64 = imin(lower, hi); return clamped; }
#[strict] fn std_abs_diff(a: i64, b: i64) -> i64 { let d: i64 = a - b; let r: i64 = iabs(d); return r; }
#[strict] fn std_signum(x: i64) -> i64 { let c: trit = x <=> 0; let s: i64 = sel_trit(c, -1, 0, 1); return s; }
#[strict] fn std_mad_clamped(x: i64, weight: i64, bias: i64, lo: i64, hi: i64) -> i64 { let raw: i64 = x * weight + bias; let lower: i64 = imax(raw, lo); let res: i64 = imin(lower, hi); return res; }
#[strict] fn std_sum_range(n: i64) -> i64 { let acc: i64 = 0; let i: i64 = 1; while i <= n { acc += i; i += 1; } return acc; }",
        ),
        "std/trit.omni" => Some(
            "#[strict] fn std_trit_mux(cond: trit, neg_v: i64, zero_v: i64, pos_v: i64) -> i64 { let r: i64 = sel_trit(cond, neg_v, zero_v, pos_v); return r; }
#[strict] fn std_consensus3(s0: trit, s1: trit, s2: trit) -> trit { let m: trit = tmaj(s0, s1, s2); return m; }
#[strict] fn std_trit_quorum5(t0: trit, t1: trit, t2: trit, t3: trit, t4: trit) -> trit { let left: trit = tmaj(t0, t1, t2); let right: trit = tmaj(t2, t3, t4); let out: trit = tmaj(left, t2, right); return out; }
#[strict] fn std_trit_score(t0: trit, t1: trit, t2: trit, weight: i64) -> i64 { let m: trit = tmaj(t0, t1, t2); let base: i64 = sel_trit(m, -100, 0, 100); return base * weight; }",
        ),
        "std/http.omni" => Some(
            "#[strict] fn std_http_is_success(status: i64) -> trit { let ge_200: trit = status >= 200; let lt_300: trit = status < 300; let ok: trit = tand(ge_200, lt_300); return ok; }
#[strict] fn std_http_pack_response(status: i64, payload: i64) -> i64 { let valid_cmp: trit = status >= 100; let code: i64 = if_pos valid_cmp { status } else { 500 }; return code * 1000000 + payload; }
#[strict] fn std_http_route_score(method_code: i64, param_id: i64) -> i64 { let m_cmp: trit = method_code <=> 1; let method_bonus: i64 = match_trit m_cmp { neg => 400, zero => 200, pos => 201 }; return method_bonus * 1000000 + param_id; }",
        ),
        "std/pack5.omni" => Some(
            "#[strict] fn std_pack5_bytes_needed(trit_count: i64) -> i64 { let valid_cmp: trit = trit_count <=> 0; let padded: i64 = trit_count + 4; let bytes: i64 = padded / 5; let out: i64 = if_pos valid_cmp { bytes } else { 0 }; return out; }
#[strict] fn std_pack5_is_valid_byte(byte_val: i64) -> trit { let ge_zero: trit = byte_val >= 0; let le_max: trit = byte_val <= 242; let ok: trit = tand(ge_zero, le_max); return ok; }
#[strict] fn std_pack5_dot10_score(dot_raw: i64, scale: i64, threshold: i64) -> i64 { let scaled: i64 = dot_raw * scale; let cmp: trit = scaled <=> threshold; let bonus: i64 = match_trit cmp { neg => -50, zero => 0, pos => 50 }; return scaled + bonus; }
#[strict] fn std_pack5_cosine_gate(dot_raw: i64, norm_bound: i64) -> trit { let hi_cmp: trit = dot_raw <=> norm_bound; let neg_bound: i64 = 0 - norm_bound; let lo_cmp: trit = dot_raw <=> neg_bound; let gate: trit = tmaj(hi_cmp, lo_cmp, 0t); return gate; }",
        ),
        "std/decision.omni" => Some(
            "#[strict] fn std_decision_band(signal: i64, lo_thresh: i64, hi_thresh: i64) -> trit { let above_hi: trit = signal <=> hi_thresh; let below_lo: trit = signal <=> lo_thresh; let is_high: trit = tand(above_hi, +1t); let is_low: trit = tor(below_lo, -1t); let out: trit = match_trit above_hi { neg => is_low, zero => 0t, pos => is_high }; return out; }
#[strict] fn std_decision_hysteresis(prev_state: trit, signal: i64, margin: i64) -> trit { let neg_margin: i64 = 0 - margin; let hi_cmp: trit = signal <=> margin; let lo_cmp: trit = signal <=> neg_margin; let next_state: trit = match_trit hi_cmp { neg => match_trit lo_cmp { neg => -1t, zero => prev_state, pos => prev_state }, zero => prev_state, pos => +1t }; return next_state; }
#[strict] #[cap(AI)] fn std_decision_weighted_hook(signal: i64, bias: i64, neg_out: i64, zero_out: i64, pos_out: i64) -> i64 { let net: i64 = signal + bias; let g: trit = egate(net); let l: trit = tlogic(net); let combined: trit = tmaj(g, l, g); let res: i64 = sel_trit(combined, neg_out, zero_out, pos_out); return res; }
#[strict] #[cap(AI)] fn std_decision_quorum_hook(s0: i64, s1: i64, s2: i64, threshold: i64) -> trit { let g0: trit = egate(s0 - threshold); let g1: trit = egate(s1 - threshold); let g2: trit = egate(s2 - threshold); let q: trit = tmaj(g0, g1, g2); return q; }",
        ),
        "std/resource.omni" => Some(
            "#[strict] fn std_arena_scoped_score(base: i64, weight: i64, offset: i64) -> i64 { let a_scope: arena = arena(1024); let r_view: i64 = borrow_r(a_scope); let prod: i64 = base * weight; let total: i64 = prod + offset + (r_view * 0); drop(r_view); drop(a_scope); return total; }
#[strict] fn std_budget_headroom(used_units: i64, limit_units: i64) -> i64 { let raw_rem: i64 = limit_units - used_units; let clamped: i64 = imax(raw_rem, 0); return clamped; }
#[strict] fn std_budget_is_within(used_units: i64, limit_units: i64, safety_margin: i64) -> trit { let effective_used: i64 = used_units + safety_margin; let cmp: trit = effective_used <= limit_units; return cmp; }
#[strict] fn std_arena_chunk_capacity(total_bytes: i64, chunk_size: i64) -> i64 { let valid_chunk: i64 = imax(chunk_size, 64); let chunks: i64 = total_bytes / valid_chunk; let clamped: i64 = imax(chunks, 1); return clamped; }",
        ),
        "pure/compiler.omni" | "spec/pure_self_host/omni_selfhost_compiler.omni" => Some(
            "#[strict]
fn pure_lex_classify_token(src_len: i64, strict_violation_code: i64, syntax_err_code: i64) -> i64 {
    let len_cmp: trit = src_len <=> 0;
    let syn_cmp: trit = syntax_err_code <=> 0;
    let strict_cmp: trit = strict_violation_code <=> 1;
    let strict_any: trit = strict_violation_code <=> 0;
    let strict_res: i64 = match_trit strict_cmp {
        neg => 0,
        zero => 2002,
        pos => 2003
    };
    let after_strict: i64 = if_pos strict_any { strict_res } else { 0 };
    let after_syn: i64 = if_pos syn_cmp { 2001 } else { after_strict };
    let final_code: i64 = match_trit len_cmp {
        neg => 2001,
        zero => 2001,
        pos => after_syn
    };
    return final_code;
}
#[strict]
fn pure_check_include_graph(include_depth: i64, has_cycle: i64, max_depth: i64) -> i64 {
    let cyc_cmp: trit = has_cycle <=> 0;
    let dep_cmp: trit = include_depth <=> max_depth;
    let dep_err: i64 = if_pos dep_cmp { 2008 } else { 0 };
    let res: i64 = if_pos cyc_cmp { 2008 } else { dep_err };
    return res;
}
#[strict]
fn pure_check_macro_hygiene(decl_arity: i64, call_arity: i64, unbound_free_vars: i64) -> i64 {
    let raw_diff: i64 = decl_arity - call_arity;
    let ar_diff: i64 = iabs(raw_diff);
    let ar_cmp: trit = ar_diff <=> 0;
    let fv_cmp: trit = unbound_free_vars <=> 0;
    let fv_err: i64 = if_pos fv_cmp { 2004 } else { 0 };
    let res: i64 = if_pos ar_cmp { 2006 } else { fv_err };
    return res;
}
#[strict]
fn pure_check_call_and_label(unknown_fn_flag: i64, immut_param_flag: i64, unresolved_lbl_flag: i64) -> i64 {
    let fn_cmp: trit = unknown_fn_flag <=> 0;
    let im_cmp: trit = immut_param_flag <=> 0;
    let lb_cmp: trit = unresolved_lbl_flag <=> 0;
    let lb_err: i64 = if_pos lb_cmp { 2009 } else { 0 };
    let im_or_lb: i64 = if_pos im_cmp { 2005 } else { lb_err };
    let res: i64 = if_pos fn_cmp { 2007 } else { im_or_lb };
    return res;
}
#[strict]
fn pure_verify_function_invariants(missing_cap_bits: i64, taint_violation_kind: i64, arena_balance: i64) -> i64 {
    let cap_cmp: trit = missing_cap_bits <=> 0;
    let taint_cmp: trit = taint_violation_kind <=> 1;
    let taint_any: trit = taint_violation_kind <=> 0;
    let arena_cmp: trit = arena_balance <=> 0;
    let taint_code: i64 = match_trit taint_cmp {
        neg => 0,
        zero => 1001,
        pos => 1002
    };
    let arena_code: i64 = match_trit arena_cmp {
        neg => 1005,
        zero => 0,
        pos => 1004
    };
    let after_taint: i64 = if_pos taint_any { taint_code } else { arena_code };
    let final_code: i64 = if_pos cap_cmp { 1003 } else { after_taint };
    return final_code;
}
#[strict]
fn pure_lower_pack5_density(trit_count: i64, max_byte_val: i64) -> i64 {
    let corrupt_cmp: trit = max_byte_val <=> 242;
    let packed_bytes: i64 = (trit_count + 4) / 5;
    let res: i64 = if_pos corrupt_cmp { 3002 } else { packed_bytes };
    return res;
}
#[strict]
fn pure_compiler_pipeline_main(src_len: i64, missing_cap_bits: i64, taint_or_arena_err: i64) -> i64 {
    let fe_code: i64 = pure_lex_classify_token(src_len, 0, 0);
    let inc_code: i64 = pure_check_include_graph(1, 0, 16);
    let mac_code: i64 = pure_check_macro_hygiene(2, 2, 0);
    let sem_code: i64 = pure_check_call_and_label(0, 0, 0);
    let fe_total: i64 = fe_code + inc_code + mac_code + sem_code;
    let fe_bad: trit = fe_total <=> 0;
    let ver_code: i64 = pure_verify_function_invariants(missing_cap_bits, taint_or_arena_err, 0);
    let final_code: i64 = if_pos fe_bad { fe_total } else { ver_code };
    return final_code;
}",
        ),
        "pure/verifier.omni" => Some(
            "#[strict]
fn omni_pure_verify_opcode_cap(fn_cap_mask: i64, req_cap_mask: i64) -> i64 {
    let covered: i64 = fn_cap_mask & req_cap_mask;
    let diff: i64 = req_cap_mask - covered;
    let cmp: trit = diff <=> 0;
    let res: i64 = if_pos cmp { 1003 } else { 0 };
    return res;
}
#[strict]
fn omni_pure_verify_taint_flow(is_ret: i64, is_sink: i64, operand_tainted: i64) -> i64 {
    let t_cmp: trit = operand_tainted <=> 0;
    let r_cmp: trit = is_ret <=> 0;
    let s_cmp: trit = is_sink <=> 0;
    let sink_err: i64 = if_pos s_cmp { 1002 } else { 0 };
    let ret_or_sink: i64 = if_pos r_cmp { 1001 } else { sink_err };
    let out: i64 = if_pos t_cmp { ret_or_sink } else { 0 };
    return out;
}
#[strict]
fn omni_pure_verify_header(missing_cap_bits: i64, taint_violation_kind: i64, arena_balance: i64) -> i64 {
    let cap_cmp: trit = missing_cap_bits <=> 0;
    let taint_cmp: trit = taint_violation_kind <=> 1;
    let taint_any: trit = taint_violation_kind <=> 0;
    let arena_cmp: trit = arena_balance <=> 0;
    let taint_code: i64 = match_trit taint_cmp {
        neg => 0,
        zero => 1001,
        pos => 1002
    };
    let arena_code: i64 = match_trit arena_cmp {
        neg => 1005,
        zero => 0,
        pos => 1004
    };
    let after_taint: i64 = if_pos taint_any { taint_code } else { arena_code };
    let final_code: i64 = if_pos cap_cmp { 1003 } else { after_taint };
    return final_code;
}",
        ),
        "pure/host_runtime.omni" | "spec/pure_self_host/host_runtime.omni" => Some(
            "#[strict]
fn omni_pure_host_check_route_caps(cap_mask: i64, needs_db_r: i64, needs_db_w: i64) -> i64 {
    let has_net: i64 = cap_mask & 4;
    let net_cmp: trit = has_net <=> 0;
    let r_cmp: trit = needs_db_r <=> 0;
    let w_cmp: trit = needs_db_w <=> 0;
    let has_r: i64 = cap_mask & 1;
    let has_w: i64 = cap_mask & 2;
    let r_ok: i64 = if_pos r_cmp { has_r } else { 1 };
    let w_ok: i64 = if_pos w_cmp { has_w } else { 1 };
    let rw_prod: i64 = r_ok * w_ok;
    let rw_cmp: trit = rw_prod <=> 0;
    let rw_status: i64 = if_pos rw_cmp { 0 } else { 1003 };
    let final_status: i64 = if_pos net_cmp { rw_status } else { 1003 };
    return final_status;
}
#[strict]
fn omni_pure_host_check_taint_sink(param_tainted: i64, auth_verified: i64, is_db_or_rsp_sink: i64) -> i64 {
    let t_cmp: trit = param_tainted <=> 0;
    let a_cmp: trit = auth_verified <=> 0;
    let s_cmp: trit = is_db_or_rsp_sink <=> 0;
    let unsan: i64 = if_pos a_cmp { 0 } else { 1002 };
    let sink_err: i64 = if_pos s_cmp { unsan } else { 0 };
    let out: i64 = if_pos t_cmp { sink_err } else { 0 };
    return out;
}
#[strict]
fn omni_pure_host_kv_apply(op_code: i64, key_id: i64, val_in: i64) -> i64 {
    let op_cmp: trit = op_code <=> 2;
    let q_val: i64 = key_id * 10;
    let res: i64 = match_trit op_cmp {
        neg => val_in,
        zero => val_in,
        pos => q_val
    };
    return res;
}
#[strict]
fn omni_pure_host_dispatch(method_code: i64, status_code: i64, payload: i64) -> i64 {
    let m_cmp: trit = method_code <=> 0;
    let s_cmp: trit = status_code <=> 199;
    let valid_status: i64 = if_pos s_cmp { status_code } else { 500 };
    let packed: i64 = valid_status * 1000000 + payload;
    let res: i64 = match_trit m_cmp {
        neg => 400000000,
        zero => 404000000,
        pos => packed
    };
    return res;
}",
        ),
        "pure/tooling.omni" => Some(
            "#[strict]
fn pure_acceptance_diff_check(rust_words: i64, pure_words: i64, hash_diff: i64) -> i64 {
    let raw_diff: i64 = rust_words - pure_words;
    let len_diff: i64 = iabs(raw_diff);
    let h_diff: i64 = iabs(hash_diff);
    let total_diff: i64 = len_diff + h_diff;
    let cmp: trit = total_diff <=> 0;
    let status: i64 = if_pos cmp { 4001 } else { 0 };
    return status;
}
#[strict]
fn pure_select_repair_strategy(err_code_id: i64, is_multi_fn: i64) -> i64 {
    let is_cap: trit = err_code_id <=> 1003;
    let multi_cmp: trit = is_multi_fn <=> 0;
    let cap_strat: i64 = if_pos multi_cmp { 2 } else { 1 };
    let res: i64 = match_trit is_cap {
        neg => err_code_id - 998,
        zero => cap_strat,
        pos => err_code_id - 997
    };
    return res;
}
#[strict]
fn pure_inspect_opcode_category(tok: i64) -> i64 {
    let is_eng: trit = tok < 32256;
    let is_op: trit = tok >= 32512;
    let op_or_meta: i64 = if_pos is_op { 6 } else { 4 };
    let category: i64 = if_pos is_eng { 1 } else { op_or_meta };
    return category;
}
#[strict]
fn pure_explain_error_stage(err_code_id: i64) -> i64 {
    let is_ver: trit = err_code_id <= 1005;
    let is_rt: trit = err_code_id >= 1013;
    let rt_or_fe: i64 = if_pos is_rt { 3 } else { 2 };
    let stage: i64 = if_pos is_ver { 1 } else { rt_or_fe };
    return stage;
}
#[strict]
fn pure_bootstrap_fixed_point_check(gen1_words: i64, gen2_words: i64, elf_hash_diff: i64) -> i64 {
    let raw_w: i64 = gen1_words - gen2_words;
    let w_diff: i64 = iabs(raw_w);
    let e_diff: i64 = iabs(elf_hash_diff);
    let w_cmp: trit = w_diff <=> 0;
    let e_cmp: trit = e_diff <=> 0;
    let elf_status: i64 = if_pos e_cmp { 5002 } else { 0 };
    let final_status: i64 = if_pos w_cmp { 5001 } else { elf_status };
    return final_status;
}
#[strict]
fn pure_inspect_module_metrics(fn_count: i64, total_words: i64, union_cap_mask: i64) -> i64 {
    let f_ok: trit = fn_count > 0;
    let w_ok: trit = total_words > 0;
    let valid: trit = tand(f_ok, w_ok);
    let score: i64 = (fn_count * 100000) + (total_words * 10) + (union_cap_mask & 1023);
    let out: i64 = if_pos valid { score } else { -1 };
    return out;
}
#[strict]
fn pure_multi_step_repair_budget(attempt: i64, max_attempts: i64, remaining_errors: i64) -> i64 {
    let err_cmp: trit = remaining_errors <=> 0;
    let budget_cmp: trit = attempt <=> max_attempts;
    let exhausted_or_step: i64 = if_pos budget_cmp { 4002 } else { remaining_errors };
    let status: i64 = if_pos err_cmp { exhausted_or_step } else { 0 };
    return status;
}",
        ),
        "pure/selfhost_suite.omni" => Some(
            "include \"std/math.omni\";
include \"std/trit.omni\";
include \"std/http.omni\";
include \"pure/compiler.omni\";
include \"pure/verifier.omni\";
include \"pure/tooling.omni\";
include \"pure/host_runtime.omni\";
macro pure_stage_gate(code) = sel_trit(code <=> 0, 1, 0, 1);
#[strict]
fn pure_selfhost_verify_fixed_point(gen1_words: i64, gen2_words: i64, suite_words: i64) -> i64 {
    let fp_st: i64 = pure_bootstrap_fixed_point_check(gen1_words, gen2_words, 0);
    let suite_st: i64 = pure_acceptance_diff_check(suite_words, suite_words, 0);
    let ver_st: i64 = omni_pure_verify_header(0, 0, 0);
    let math_c: i64 = std_mad_clamped(10, 2, 5, 0, 100);
    let q: trit = std_trit_quorum5(+1t, +1t, -1t, +1t, +1t);
    let q_bonus: i64 = std_trit_mux(q, -5, 0, 5);
    let http_ok: trit = std_http_is_success(200);
    let http_bonus: i64 = std_trit_mux(http_ok, 0, 0, 10);
    let err_sum: i64 = fp_st + suite_st + ver_st;
    let err_cmp: trit = err_sum <=> 0;
    let ok_score: i64 = math_c + q_bonus + http_bonus;
    let out: i64 = if_pos err_cmp { err_sum } else { ok_score };
    return out;
}
#[strict]
#[cap(AI)]
fn pure_selfhost_suite_entry(src_len: i64, cap_mask: i64, payload: i64) -> i64 {
    let clamped_len: i64 = std_clamp(src_len, 1, 65536);
    let comp_status: i64 = pure_compiler_pipeline_main(clamped_len, 0, 0);
    let host_cap_status: i64 = omni_pure_host_check_route_caps(cap_mask, 1, 1);
    let diff_status: i64 = pure_acceptance_diff_check(clamped_len, clamped_len, 0);
    let strat_id: i64 = pure_select_repair_strategy(1003, 1);
    let op_cat: i64 = pure_inspect_opcode_category(32565);
    let http_packed: i64 = omni_pure_host_dispatch(1, 200, payload);
    let delta: i64 = http_packed - 200000000;
    let gate: trit = egate(delta);
    let bonus: i64 = std_trit_mux(gate, -10, 0, 10);
    let errs: i64 = comp_status + host_cap_status + diff_status;
    let err_cmp: trit = errs <=> 0;
    let ok_val: i64 = http_packed + strat_id + op_cat + bonus;
    let res: i64 = if_pos err_cmp { errs } else { ok_val };
    return res;
}",
        ),
        "pure/programs/ternary_neural_router.omni" => Some(
            "include \"std/math.omni\";
include \"std/trit.omni\";
macro router_margin(a, b) = (a * 3) - (b * 2);
#[strict]
#[cap(AI)]
fn route_layer_score(bias: i64, scale: i64) -> i64 {
    let w_pos: bytes = pack5_bytes([+1t, +1t, 0t, +1t, -1t, +1t, +1t, 0t, +1t, +1t]);
    let w_neg: bytes = pack5_bytes([-1t, 0t, +1t, -1t, +1t, 0t, -1t, +1t, 0t, -1t]);
    let d_pos: i64 = pack5_tdot(w_pos, w_pos);
    let d_cross: i64 = pack5_tdot(w_pos, w_neg);
    let g_pos: trit = pack5_gate(w_pos);
    let g_bonus: i64 = std_trit_mux(g_pos, -20, 0, 20);
    let m: i64 = router_margin(d_pos, d_cross);
    let raw: i64 = m * scale + bias + g_bonus;
    let clamped: i64 = std_clamp(raw, -500, 500);
    return clamped;
}
#[strict]
#[cap(AI)]
fn route_quorum_gate(score_a: i64, score_b: i64, threshold: i64) -> trit {
    let da: i64 = score_a - threshold;
    let db: i64 = score_b - threshold;
    let sum_ab: i64 = score_a + score_b;
    let d_sum: i64 = sum_ab - threshold;
    let ga: trit = egate(da);
    let gb: trit = egate(db);
    let gc: trit = tlogic(d_sum);
    let q: trit = std_trit_quorum5(ga, gb, gc, +1t, ga);
    return q;
}
#[strict]
#[cap(AI)]
#[step_budget(10000)]
#[mem_budget(16384)]
fn main(input_signal: i64, threshold: i64) -> i64 {
    let s1: i64 = route_layer_score(input_signal, 4);
    let half_sig: i64 = input_signal / 2;
    let s2: i64 = route_layer_score(half_sig, 2);
    let gate: trit = route_quorum_gate(s1, s2, threshold);
    let bonus: i64 = std_trit_score(gate, +1t, gate, 3);
    let total: i64 = s1 + s2 + bonus;
    return total;
}",
        ),
        "pure/programs/arena_audit_pipeline.omni" => Some(
            "include \"std/math.omni\";
include \"std/trit.omni\";
#[strict]
#[cap(AI)]
fn compute_arena_window(limit: i64, stride: i64) -> i64 {
    let a_main: arena = arena(4096);
    let r_view: arena = borrow_r(a_main);
    let acc: i64 = 0;
    let i: i64 = 1;
    while i <= limit {
        let step_val: i64 = std_mad_clamped(i, stride, 3, 0, 200);
        acc += step_val;
        i += 1;
    }
    let gate_in: i64 = acc - 50;
    let g: trit = egate(gate_in);
    let adj: i64 = std_trit_mux(g, -10, 0, 10);
    drop(r_view);
    drop(a_main);
    return acc + adj;
}
#[strict]
#[cap(DB_W, DOM, AI)]
fn sanitize_and_score_payload(base_score: i64) -> i64 {
    let raw_lit: str = \"125\";
    let sql_ok: str = san_sql(raw_lit);
    let dom_ok: str = san_dom(sql_ok);
    let prm_ok: str = san_prm(dom_ok);
    let parsed_n: i64 = s2i(prm_ok);
    let combined: i64 = base_score + parsed_n;
    return combined;
}
#[strict]
#[cap(DB_W, DOM, AI)]
#[step_budget(20000)]
#[mem_budget(32768)]
fn main(limit: i64, stride: i64) -> i64 {
    let clamped_lim: i64 = std_clamp(limit, 1, 20);
    let win_score: i64 = compute_arena_window(clamped_lim, stride);
    let final_score: i64 = sanitize_and_score_payload(win_score);
    return final_score;
}",
        ),
        "pure/programs/policy_consensus_engine.omni" => Some(
            "include \"std/math.omni\";
include \"std/trit.omni\";
include \"std/http.omni\";
include \"pure/verifier.omni\";
macro risk_delta(val, limit) = val - limit;
#[strict]
#[cap(AI)]
fn evaluate_agent_vote(metric: i64, soft_limit: i64, hard_limit: i64) -> trit {
    let d_soft: i64 = risk_delta(metric, soft_limit);
    let d_hard: i64 = risk_delta(metric, hard_limit);
    let g_soft: trit = egate(d_soft);
    let g_hard: trit = tlogic(d_hard);
    let inv_soft: trit = !g_soft;
    let inv_hard: trit = !g_hard;
    let vote: trit = std_consensus3(inv_soft, inv_hard, +1t);
    return vote;
}
#[strict]
#[cap(AI)]
#[step_budget(15000)]
#[mem_budget(16384)]
fn main(load_pct: i64, latency_ms: i64, fn_caps: i64) -> i64 {
    let cap_err: i64 = omni_pure_verify_opcode_cap(fn_caps, 16);
    let cap_cmp: trit = cap_err <=> 0;
    let v_load: trit = evaluate_agent_vote(load_pct, 70, 90);
    let v_lat: trit = evaluate_agent_vote(latency_ms, 100, 250);
    let quorum: trit = std_trit_quorum5(v_load, v_lat, +1t, v_load, v_lat);
    let q_score: i64 = std_trit_score(quorum, v_load, v_lat, 2);
    let abs_gap: i64 = std_abs_diff(load_pct, latency_ms);
    let payload: i64 = std_clamp(q_score + abs_gap, 0, 999999);
    let status: i64 = std_trit_mux(quorum, 503, 202, 200);
    let http_out: i64 = std_http_pack_response(status, payload);
    let res: i64 = if_pos cap_cmp { cap_err } else { http_out };
    return res;
}",
        ),
        "pure/programs/kv_ledger_service.omni" => Some(
            "include \"std/math.omni\";
include \"std/trit.omni\";
#[strict]
#[cap(NET, DB_R, DB_W, AUTH, AI)]
route POST /ledger/:id (req: i64) {
    let raw_id: i64 = req.param(\"id\");
    let clean_id: i64 = auth_verify(raw_id);
    let bounded_id: i64 = std_clamp(clean_id, 1, 10000);
    let base_deposit: i64 = bounded_id * 10 + 500;
    let delta: i64 = base_deposit - 600;
    let g: trit = egate(delta);
    let tier_bonus: i64 = std_trit_mux(g, 0, 25, 100);
    let final_balance: i64 = base_deposit + tier_bonus;
    let saved: i64 = kv_put(bounded_id, final_balance);
    respond(201, saved);
}
#[strict]
#[cap(NET, DB_R, AUTH)]
route GET /ledger/:id (req: i64) {
    let raw_id: i64 = req.param(\"id\");
    let clean_id: i64 = auth_verify(raw_id);
    let cur_bal: i64 = db_get(clean_id);
    let audit_row: i64 = db_query(\"SELECT balance FROM ledger WHERE id = ?\", clean_id);
    respond(200, cur_bal + audit_row);
}",
        ),
        "pure/programs/compiler_self_audit.omni" => Some(
            "include \"std/math.omni\";
include \"std/trit.omni\";
include \"std/http.omni\";
include \"pure/compiler.omni\";
include \"pure/verifier.omni\";
include \"pure/tooling.omni\";
include \"pure/host_runtime.omni\";
#[strict]
#[cap(AI)]
#[step_budget(25000)]
#[mem_budget(32768)]
fn main(src_words: i64, trit_len: i64, err_probe: i64) -> i64 {
    let c_st: i64 = pure_compiler_pipeline_main(src_words, 0, 0);
    let v_st: i64 = omni_pure_verify_header(0, 0, 0);
    let p5_bytes: i64 = pure_lower_pack5_density(trit_len, 240);
    let diff_st: i64 = pure_acceptance_diff_check(src_words, src_words, 0);
    let fp_st: i64 = pure_bootstrap_fixed_point_check(src_words, src_words, 0);
    let strat: i64 = pure_select_repair_strategy(err_probe, 1);
    let stg: i64 = pure_explain_error_stage(err_probe);
    let cat: i64 = pure_inspect_opcode_category(32565);
    let h_cap: i64 = omni_pure_host_check_route_caps(7, 1, 1);
    let kv_val: i64 = omni_pure_host_kv_apply(3, 50, 0);
    let err_total: i64 = c_st + v_st + diff_st + fp_st + h_cap;
    let err_cmp: trit = err_total <=> 0;
    let delta: i64 = kv_val - 100;
    let g: trit = egate(delta);
    let g_bonus: i64 = std_trit_mux(g, -10, 0, 10);
    let digest: i64 = (p5_bytes * 1000) + (strat * 100) + (cat * 10) + stg + kv_val + g_bonus;
    let out: i64 = if_pos err_cmp { err_total } else { digest };
    return out;
}",
        ),
        "pure/programs/pack5_decision_controller.omni" => Some(
            "include \"std/pack5.omni\";
include \"std/decision.omni\";
include \"std/resource.omni\";
#[strict]
#[cap(AI)]
#[step_budget(24000)]
#[mem_budget(8192)]
fn main(signal_a: i64, signal_b: i64, used_steps: i64) -> i64 {
    let w_vec: bytes = pack5_bytes([+1t, +1t, 0t, +1t, -1t, +1t, +1t, 0t, +1t, +1t]);
    let w_dot: i64 = pack5_tdot(w_vec, w_vec);
    let p_bytes: i64 = std_pack5_bytes_needed(50);
    let b_ok: trit = std_pack5_is_valid_byte(215);
    let dot_sc: i64 = std_pack5_dot10_score(w_dot, 2, 10);
    let cos_g: trit = std_pack5_cosine_gate(w_dot, 5);
    let band_t: trit = std_decision_band(signal_a, 10, 30);
    let hyst_t: trit = std_decision_hysteresis(band_t, signal_b, 15);
    let hook_val: i64 = std_decision_weighted_hook(signal_a + signal_b, -20, -100, 0, 100);
    let quorum_t: trit = std_decision_quorum_hook(signal_a, signal_b, signal_a + 10, 20);
    let arena_sc: i64 = std_arena_scoped_score(p_bytes, 10, hook_val);
    let headroom: i64 = std_budget_headroom(used_steps, 1000);
    let budget_ok: trit = std_budget_is_within(used_steps, 1000, 50);
    let chunks: i64 = std_arena_chunk_capacity(4096, 512);
    let gate_all: trit = tmaj(b_ok, cos_g, quorum_t);
    let hyst_bonus: i64 = sel_trit(hyst_t, -25, 0, 25);
    let gate_bonus: i64 = sel_trit(gate_all, -10, 0, 40);
    let total: i64 = dot_sc + arena_sc + (headroom / 10) + chunks + hyst_bonus + gate_bonus;
    let final_out: i64 = if_pos budget_ok { total } else { -1 };
    return final_out;
}",
        ),
        _ => None,
    }
}

/// Resolves `include "other.omni";` / `mod "other.omni";` directives from an in-memory map, built-in `std/*` library, or filesystem directory.
pub fn resolve_omni_surface_includes(
    src: &str,
    base_dir: Option<&std::path::Path>,
    vfs: Option<&BTreeMap<String, String>>,
) -> Result<String, String> {
    fn resolve_inner(
        cur_src: &str,
        cur_dir: Option<&std::path::Path>,
        vfs: Option<&BTreeMap<String, String>>,
        stack: &mut Vec<String>,
    ) -> Result<String, String> {
        if !cur_src.contains("include") && !cur_src.contains("mod ") && !cur_src.contains("mod\"") {
            return Ok(String::from(cur_src));
        }
        let tokens = tokenize_omni_surface(cur_src)?;
        let mut parser = SurfaceParser::new(tokens);
        let (includes, _fns) = parser.parse_module()?;
        if includes.is_empty() {
            return Ok(String::from(cur_src));
        }

        let mut combined = String::new();
        for (inc_path, line, col) in includes {
            if stack.contains(&inc_path) {
                return Err(format!(
                    "error[CIRCULAR_INCLUDE] line {}, col {}: Circular include detected for '{}'",
                    line, col, inc_path
                ));
            }
            let inc_content = if let Some(map) = vfs.and_then(|m| m.get(&inc_path).map(|s| (m, s))) {
                map.1.clone()
            } else if let Some(std_src) = lookup_omni_stdlib_module(&inc_path) {
                String::from(std_src)
            } else if let Some(dir) = cur_dir {
                let full_p = dir.join(&inc_path);
                fs::read_to_string(&full_p).map_err(|e| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Failed to read included file '{}': {}",
                        line, col, full_p.display(), e
                    )
                })?
            } else {
                fs::read_to_string(&inc_path).map_err(|e| {
                    format!(
                        "error[SYNTAX_ERROR] line {}, col {}: Failed to read included file '{}': {}",
                        line, col, inc_path, e
                    )
                })?
            };
            stack.push(inc_path.clone());
            let expanded = resolve_inner(&inc_content, cur_dir, vfs, stack)?;
            stack.pop();
            combined.push_str(&expanded);
            combined.push('\n');
        }

        for raw_line in cur_src.lines() {
            let trimmed = raw_line.trim_start();
            if trimmed.starts_with("include ") || trimmed.starts_with("include\"") || trimmed.starts_with("mod ") {
                continue;
            }
            combined.push_str(raw_line);
            combined.push('\n');
        }
        Ok(combined)
    }

    let mut stack = Vec::new();
    resolve_inner(src, base_dir, vfs, &mut stack)
}

pub(crate) fn compile_omni_surface_to_asm_inner(src: &str, strict: bool) -> Result<String, String> {
    let effective_strict = strict || src.contains("#[strict]");
    let tokens = tokenize_omni_surface_inner(src, effective_strict)?;
    let mut parser = SurfaceParser::new_with_strict(tokens, effective_strict);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return compile_omni_surface_to_asm_inner(&expanded, effective_strict);
    }

    let multi_fn = fns.len() > 1;
    let mut fn_sigs: BTreeMap<String, (u16, usize, String)> = BTreeMap::new();
    let mut assigned_ids: Vec<u16> = Vec::with_capacity(fns.len());

    for (idx, f) in fns.iter().enumerate() {
        let fid = if let Some(id_attr) = f.fn_id_attr {
            id_attr
        } else if !multi_fn || idx + 1 == fns.len() {
            0
        } else {
            (idx + 1) as u16
        };
        let fn_strict = effective_strict || f.strict_attr;
        let ret_asm = String::from(map_omni_surface_type_inner(&f.ret_ty, f.ret_line, f.ret_col, fn_strict)?);
        for p in &f.params {
            let _ = map_omni_surface_type_inner(&p.ty, p.line, p.col, fn_strict)?;
        }
        if fn_sigs.contains_key(&f.name) {
            return Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Duplicate function definition '{}'",
                f.ret_line, f.ret_col, f.name
            ));
        }
        fn_sigs.insert(f.name.clone(), (fid, f.params.len(), ret_asm));
        assigned_ids.push(fid);
    }

    let mut out = Vec::new();
    for (idx, f) in fns.iter().enumerate() {
        let fid = assigned_ids[idx];
        let fn_strict = effective_strict || f.strict_attr;
        if multi_fn {
            out.push(format!("FN_DEF {}", fid));
        } else if let Some(explicit_id) = f.fn_id_attr {
            out.push(format!("FN_ID {}", explicit_id));
        }
        if f.cap_mask != 0 {
            out.push(format!("CAP_MASK {}", f.cap_mask));
        }
        for (p_idx, p) in f.params.iter().enumerate() {
            let ty_str = map_omni_surface_type_inner(&p.ty, p.line, p.col, fn_strict)?;
            let taint_str = if p.untrusted { "UNTRUSTED" } else { "SAFE" };
            out.push(format!("PARAM ${} {} {}", p_idx, ty_str, taint_str));
        }
        let ret_ty_str = map_omni_surface_type_inner(&f.ret_ty, f.ret_line, f.ret_col, fn_strict)?;
        let ret_ty_tok = parse_asm_ty_tok(ret_ty_str);
        out.push(format!("RET_TY {}", ret_ty_str));

        let mut ctx = SurfaceLowerCtx::new(&f.params, fn_sigs.clone(), ret_ty_tok, true);
        if f.is_route {
            let method_code = match f.route_method.as_deref() {
                Some("GET") => 1,
                Some("POST") => 2,
                Some("PUT") => 3,
                Some("DELETE") => 4,
                _ => 0,
            };
            let path_hash = fnv1a_32_positive(f.route_path.as_deref().unwrap_or("/"));
            let r_rt = ctx.alloc_reg(f.ret_line, f.ret_col)?;
            ctx.emit(format!("{} = ROUTE {} {}", r_rt, method_code, path_hash))?;
        }
        for stmt in &f.body {
            ctx.lower_stmt(stmt)?;
        }
        ctx.check_unresolved_labels()?;
        out.extend(ctx.lines);
    }

    let mut joined = out.join("\n");
    joined.push('\n');
    Ok(joined)
}

/// Lowers an Omni surface language program (single-function, multi-function, or `route`) to canonical `.omni_asm`.
pub fn compile_omni_surface_to_asm(src: &str) -> Result<String, String> {
    compile_omni_surface_to_asm_inner(src, false)
}

/// Maps binary `u16` stream instruction offsets and runtime budget traps back to 1-based `(line, col)` surface coordinates.
#[derive(Clone, Debug)]
pub struct SurfaceSourceMap {
    pub offset_to_span: BTreeMap<usize, (usize, usize)>,
    pub budget_span: (usize, usize),
    pub pack5_span: Option<(usize, usize)>,
    pub default_span: (usize, usize),
}

impl Default for SurfaceSourceMap {
    fn default() -> Self {
        Self {
            offset_to_span: BTreeMap::new(),
            budget_span: (1, 1),
            pack5_span: None,
            default_span: (1, 1),
        }
    }
}

/// Injects a 1-based `line <L>, col <C>` span into any verifier (`Offset N: ...`), `@PACK5` (`CORRUPT_PACK5`), or runtime (`BUDGET_EXCEEDED`) error string.
pub fn annotate_omni_error_with_sourcemap(err: &str, sm: &SurfaceSourceMap) -> String {
    if err.contains("line ") && err.contains("col ") {
        return String::from(err);
    }
    let mut span = sm.default_span;
    if let Some(off_idx) = err.find("Offset ") {
        let rest = &err[off_idx + 7..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(off) = digits.parse::<usize>() {
            if let Some(&mapped) = sm.offset_to_span.get(&off) {
                span = mapped;
            }
        }
    } else if err.contains("BUDGET_EXCEEDED") || err.contains("budget exceeded") {
        span = sm.budget_span;
    } else if err.contains("CORRUPT_PACK5") || err.contains("PACK5_SENTINEL_OR_CORRUPT_BYTE") {
        span = sm.pack5_span.unwrap_or(sm.default_span);
    }
    let line = span.0.max(1);
    let col = span.1.max(1);
    if let Some(e_start) = err.find("error[") {
        if let Some(rel_end) = err[e_start..].find(']') {
            let bracket_end = e_start + rel_end + 1;
            return format!(
                "{} line {}, col {}{}",
                &err[..bracket_end],
                line,
                col,
                &err[bracket_end..]
            );
        }
    }
    format!("line {}, col {}: {}", line, col, err)
}

/// Compiles an Omni surface program into a `Vec<u16>` token stream alongside its 1-based `SurfaceSourceMap`.
pub fn compile_omni_surface_to_u16_with_sourcemap(
    src: &str,
    strict: bool,
) -> Result<(Vec<u16>, SurfaceSourceMap), String> {
    let effective_strict = strict || src.contains("#[strict]");
    let tokens = tokenize_omni_surface_inner(src, effective_strict)?;
    let mut parser = SurfaceParser::new_with_strict(tokens, effective_strict);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return compile_omni_surface_to_u16_with_sourcemap(&expanded, effective_strict);
    }

    let multi_fn = fns.len() > 1;
    let mut fn_sigs: BTreeMap<String, (u16, usize, String)> = BTreeMap::new();
    let mut assigned_ids: Vec<u16> = Vec::with_capacity(fns.len());

    for (idx, f) in fns.iter().enumerate() {
        let fid = if let Some(id_attr) = f.fn_id_attr {
            id_attr
        } else if !multi_fn || idx + 1 == fns.len() {
            0
        } else {
            (idx + 1) as u16
        };
        let fn_strict = effective_strict || f.strict_attr;
        let ret_asm = String::from(map_omni_surface_type_inner(&f.ret_ty, f.ret_line, f.ret_col, fn_strict)?);
        for p in &f.params {
            let _ = map_omni_surface_type_inner(&p.ty, p.line, p.col, fn_strict)?;
        }
        if fn_sigs.contains_key(&f.name) {
            return Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Duplicate function definition '{}'",
                f.ret_line, f.ret_col, f.name
            ));
        }
        fn_sigs.insert(f.name.clone(), (fid, f.params.len(), ret_asm));
        assigned_ids.push(fid);
    }

    let mut sm = SurfaceSourceMap::default();
    if let Some(first_fn) = fns.first() {
        sm.default_span = (first_fn.fn_line.max(1), first_fn.fn_col.max(1));
        sm.budget_span = sm.default_span;
    }

    let mut completed_fns: Vec<(
        u16,
        Vec<(u16, u16, u16)>,
        u16,
        CapMask,
        Vec<Vec<u16>>,
        Vec<(usize, usize)>,
    )> = Vec::with_capacity(fns.len());

    for (idx, f) in fns.iter().enumerate() {
        if let Some(bs) = f.budget_span {
            sm.budget_span = (bs.0.max(1), bs.1.max(1));
        } else {
            for st in &f.body {
                if matches!(st, SurfaceStmt::While(_, _, _, _) | SurfaceStmt::DoWhile(_, _, _, _)) {
                    let sp = st.span();
                    sm.budget_span = (sp.0.max(1), sp.1.max(1));
                    break;
                }
            }
        }
        let fid = assigned_ids[idx];
        let fn_strict = effective_strict || f.strict_attr;
        let mut params = Vec::with_capacity(f.params.len());
        for (p_idx, p) in f.params.iter().enumerate() {
            let ty_str = map_omni_surface_type_inner(&p.ty, p.line, p.col, fn_strict)?;
            let ty_tok = parse_asm_ty_tok(ty_str);
            params.push((PAR_BASE + (p_idx as u16), ty_tok, pack_tc(p.untrusted, CAP_ID_PURE)));
        }
        let ret_ty_str = map_omni_surface_type_inner(&f.ret_ty, f.ret_line, f.ret_col, fn_strict)?;
        let ret_ty_tok = parse_asm_ty_tok(ret_ty_str);

        let mut ctx = SurfaceLowerCtx::new(&f.params, fn_sigs.clone(), ret_ty_tok, false);
        ctx.cur_span = (f.fn_line.max(1), f.fn_col.max(1));
        if f.is_route {
            let method_code = match f.route_method.as_deref() {
                Some("GET") => 1,
                Some("POST") => 2,
                Some("PUT") => 3,
                Some("DELETE") => 4,
                _ => 0,
            };
            let path_hash = fnv1a_32_positive(f.route_path.as_deref().unwrap_or("/"));
            let r_rt = ctx.alloc_reg(f.fn_line, f.fn_col)?;
            ctx.cur_span = (f.fn_line.max(1), f.fn_col.max(1));
            ctx.emit(format!("{} = ROUTE {} {}", r_rt, method_code, path_hash))?;
        }
        for stmt in &f.body {
            ctx.lower_stmt(stmt)?;
        }
        ctx.check_unresolved_labels()?;
        completed_fns.push((fid, params, ret_ty_tok, f.cap_mask, ctx.packets, ctx.pkt_spans));
    }

    if completed_fns.len() == 1 {
        let (fn_id, params, ret_ty, cap_mask, ins_packets, pkt_spans) = completed_fns.pop().unwrap();
        let mut cur_off = 7 + 3 * params.len();
        for (pkt, sp) in ins_packets.iter().zip(pkt_spans.into_iter()) {
            sm.offset_to_span.insert(cur_off, sp);
            if sm.pack5_span.is_none()
                && pkt.get(1).copied().map_or(false, |op| {
                    matches!(op, 0x7F30 | 0x7F31 | 0x7F34 | 0x7F35 | 0x7F3F)
                })
            {
                sm.pack5_span = Some(sp);
            }
            cur_off += pkt.len();
        }
        let stream = pack_fn_module(&[], fn_id, &params, ret_ty, cap_mask, &ins_packets);
        Ok((stream, sm))
    } else {
        let mut stream = vec![TOK_MOD_BEGIN];
        for (f_id, f_params, f_ret, f_cap, f_ins, f_spans) in completed_fns {
            stream.push(TOK_FN_DEF);
            stream.push(f_id);
            stream.push(f_params.len() as u16);
            for (pt, pty, ptc) in f_params {
                stream.push(pt);
                stream.push(pty);
                stream.push(ptc);
            }
            stream.push(f_ret);
            stream.push(f_cap);
            stream.push(f_ins.len() as u16);
            for (pkt, sp) in f_ins.into_iter().zip(f_spans.into_iter()) {
                sm.offset_to_span.insert(stream.len(), sp);
                if sm.pack5_span.is_none()
                    && pkt.get(1).copied().map_or(false, |op| {
                        matches!(op, 0x7F30 | 0x7F31 | 0x7F34 | 0x7F35 | 0x7F3F)
                    })
                {
                    sm.pack5_span = Some(sp);
                }
                stream.extend_from_slice(&pkt);
            }
        }
        stream.push(TOK_MOD_END);
        Ok((stream, sm))
    }
}

pub(crate) fn compile_omni_surface_to_u16_inner(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    compile_omni_surface_to_u16_with_sourcemap(src, strict).map(|(s, _)| s)
}

/// Compiles an Omni surface language program directly into a `Vec<u16>` token stream without
/// materializing or re-parsing an intermediate `.omni_asm` text file.
pub fn compile_omni_surface_to_u16(src: &str) -> Result<Vec<u16>, String> {
    compile_omni_surface_to_u16_inner(src, false)
}

/// Compiles an Omni surface language program in strict AI-first single-form mode (`#[strict]`),
/// requiring explicit type annotations on every `let` (`let x: <ty> = ...;`) and rejecting
/// non-canonical trit aliases (`TRIT_POS`), type aliases (`int`/`tvec`), taint prefixes (`untrusted_i64`),
/// and `mod "..."` directives with `error[BAD_TYPE]` or `error[AMBIGUOUS_GRAMMAR]`.
pub fn compile_omni_surface_strict_to_u16(src: &str) -> Result<Vec<u16>, String> {
    compile_omni_surface_to_u16_inner(src, true)
}

/// Extracts any `#[step_budget(N)]` and `#[mem_budget(N)]` attributes declared on surface functions.
pub fn extract_omni_surface_budgets(src: &str) -> (Option<u64>, Option<usize>) {
    let Ok(tokens) = tokenize_omni_surface(src) else {
        return (None, None);
    };
    let mut parser = SurfaceParser::new(tokens);
    let Ok((includes, fns)) = parser.parse_module() else {
        return (None, None);
    };
    if !includes.is_empty() {
        if let Ok(expanded) = resolve_omni_surface_includes(src, None, None) {
            return extract_omni_surface_budgets(&expanded);
        }
    }
    let mut step_b: Option<u64> = None;
    let mut mem_b: Option<usize> = None;
    for f in fns {
        if let Some(sb) = f.step_budget_attr {
            step_b = Some(step_b.map_or(sb, |cur| cur.min(sb)));
        }
        if let Some(mb) = f.mem_budget_attr {
            mem_b = Some(mem_b.map_or(mb, |cur| cur.min(mb)));
        }
    }
    (step_b, mem_b)
}

/// Compiles a `.omni` file from disk, resolving any relative `include "..."` directives,
/// through the primary Pure-Omni surface frontend (`compile_omni_surface_via_pure_omni_frontend`)
/// and verifies the emitted `Vec<u16>` stream with `verify_module_via_phase7_pure_omni_kernel`.
pub fn compile_omni_surface_file_to_u16(path: &str) -> Result<Vec<u16>, String> {
    let raw_src = fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    let parent_dir = std::path::Path::new(path).parent();
    let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None)?;
    let u16_stream = compile_omni_surface_via_pure_omni_frontend(&expanded, expanded.contains("#[strict]"))?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Links multiple in-memory `.omni` surface modules and compiles them into a single `Vec<u16>` token stream
/// via the primary Pure-Omni surface frontend.
pub fn link_omni_surface_modules(modules: &[&str]) -> Result<Vec<u16>, String> {
    let joined = modules.join("\n");
    compile_omni_surface_via_pure_omni_frontend(&joined, joined.contains("#[strict]"))
}

/// Parses a CLI string argument into an `OmniValue` (`+1t`/`0t`/`-1t` -> `Trit`, integer -> `Int`, else `Str`).
pub fn parse_cli_omni_value(a_str: &str) -> OmniValue {
    match a_str {
        "+1t" | "TRIT_POS" => OmniValue::Trit(1),
        "0t" | "TRIT_ZERO" => OmniValue::Trit(0),
        "-1t" | "TRIT_NEG" => OmniValue::Trit(-1),
        _ => {
            if let Ok(n) = a_str.parse::<i64>() {
                OmniValue::Int(n)
            } else {
                OmniValue::Str(String::from(a_str))
            }
        }
    }
}

/// Wraps a bare expression or multi-statement snippet in a full `#[cap(...)] fn main(...) -> <ty>` function if it does not already contain `fn ` or `route `.
pub fn wrap_omni_surface_snippet(snippet: &str) -> String {
    let t_check = snippet.trim_start();
    if snippet.contains("fn ") || t_check.starts_with("route ") || snippet.contains("\nroute ") || snippet.contains(" route ") {
        return String::from(snippet);
    }
    let trimmed = snippet.trim().trim_end_matches(';').trim();
    let bytes = trimmed.as_bytes();
    let mut in_str = false;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut stmt_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if b == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => brace_depth += 1,
            b'}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 {
                    let cur_stmt = trimmed[stmt_start..=i].trim_start();
                    if cur_stmt.starts_with("while ") || cur_stmt.starts_with("while(") {
                        stmt_start = i + 1;
                    }
                }
            }
            b'(' => paren_depth += 1,
            b')' => paren_depth = paren_depth.saturating_sub(1),
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            b';' => {
                if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 {
                    stmt_start = i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let prefix = trimmed[..stmt_start].trim();
    let raw_tail = trimmed[stmt_start..].trim();
    let tail = if let Some(stripped) = raw_tail.strip_prefix("return ") {
        stripped.trim()
    } else if raw_tail.is_empty() {
        "0"
    } else {
        raw_tail
    };

    let infer_expr_ty = |expr: &str| -> &'static str {
        let e = expr.trim();
        if e.starts_with('"')
            || e.starts_with("san_sql(")
            || e.starts_with("san_dom(")
            || e.starts_with("san_prm(")
            || e.starts_with("scat(")
            || e.starts_with("sconc(")
            || e.starts_with("i2s(")
        {
            "str"
        } else if e.starts_with('[') || e.starts_with("unpack5(") {
            "trit_vec"
        } else if e.starts_with("pack5_bytes(") || e.starts_with("anew(") {
            "bytes"
        } else if e.contains("<=>")
            || e.contains("==")
            || e.contains("!=")
            || e.contains("<=")
            || e.contains(">=")
            || e.contains(" < ")
            || e.contains(" > ")
            || e.starts_with("egate(")
            || e.starts_with("decision_hook(")
            || e.starts_with("pack5_gate(")
            || e.starts_with("tlogic(")
            || e.starts_with("tmaj(")
            || e.starts_with("tand(")
            || e.starts_with("tor(")
            || e.starts_with("txor(")
            || e.starts_with("tcmp(")
            || e == "+1t"
            || e == "0t"
            || e == "-1t"
        {
            "trit"
        } else {
            "i64"
        }
    };

    let ret_ty = infer_expr_ty(tail);
    if prefix.is_empty() {
        format!(
            "#[cap(AI, DB_R, DB_W, NET, DOM, VISION, AUDIO, AUTH, FS, SYS)] fn main() -> {} {{ let __res = {}; return __res; }}",
            ret_ty, tail
        )
    } else {
        format!(
            "#[cap(AI, DB_R, DB_W, NET, DOM, VISION, AUDIO, AUTH, FS, SYS)] fn main() -> {} {{ {} let __res = {}; return __res; }}",
            ret_ty, prefix, tail
        )
    }
}

/// Compiles, verifies, and executes an Omni surface program with explicit step and memory budgets.
pub fn execute_omni_surface_program_with_budgets(
    src: &str,
    args: Vec<OmniValue>,
    step_budget: Option<u64>,
    mem_budget: Option<usize>,
) -> Result<OmniValue, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let eff_steps = match (step_budget, attr_steps) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let eff_mem = match (mem_budget, attr_mem) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(&wrapped, false)?;
    for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let entry_fn_idx = if module.functions.iter().any(|f| f.fn_idx == 0) {
        0
    } else {
        module.functions.last().map(|f| f.fn_idx).ok_or("No functions in module")?
    };
    let mut vm = OmniBinaryVm::new();
    if let Some(sb) = eff_steps {
        vm.step_limit = sb;
    }
    if let Some(mb) = eff_mem {
        vm.memory_budget_bytes = mb;
    }
    vm.execute_function(&module, entry_fn_idx, args)
        .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))
}

/// Executes an Omni surface program using the AOT hot path (`compile_pure_u16_to_wasm` ->
/// `execute_wasm_module_i64_with_budgets`) whenever the module is an AOT-eligible numeric/ternary/
/// `@PACK5` hot path returning `i64` or `trit`, falling back to `OmniBinaryVm` under explicit budgets
/// only for high-level string, composite-return, host-ABI (`route`/DB/KV), or multi-function modules.
/// Returns `(result_value, "aot" | "vm")`.
pub fn execute_omni_surface_aot_first(
    src: &str,
    args: Vec<OmniValue>,
) -> Result<(OmniValue, &'static str), String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(&wrapped, false)?;
    for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    for func in &module.functions {
        let rep = verify_binary_function(func);
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, func.fn_idx, rep.errors));
        }
    }
    let entry_fn_idx = if module.functions.iter().any(|f| f.fn_idx == 0) {
        0
    } else {
        module.functions.last().map(|f| f.fn_idx).ok_or("No functions in module")?
    };
    let entry_ret_ty_tok = module
        .functions
        .iter()
        .find(|f| f.fn_idx == entry_fn_idx)
        .map(|f| f.ret_ty_tok)
        .unwrap_or(TY_I64);

    let all_scalar_args = args.iter().all(|a| matches!(a, OmniValue::Int(_) | OmniValue::Trit(_)));
    let scalar_ret = matches!(entry_ret_ty_tok, TY_I64 | TY_TRIT);

    if is_module_aot_hot_path(&module) && all_scalar_args && scalar_ret {
        let step_b = attr_steps.unwrap_or(DEFAULT_WASM_STEP_LIMIT);
        let mem_b = attr_mem.unwrap_or(DEFAULT_WASM_MEMORY_BUDGET_BYTES);
        let mut static_alloc_bytes = 0usize;
        for func in &module.functions {
            for ins in &func.instructions {
                if ins.op_tok == 0x7F01 {
                    if let Some(BinOperand::ImmInt(cap)) = ins.operands.first() {
                        if *cap > 0 {
                            static_alloc_bytes = static_alloc_bytes.saturating_add(*cap as usize);
                        }
                    }
                }
            }
        }
        if static_alloc_bytes > mem_b {
            let raw_err = format!(
                "error[BUDGET_EXCEEDED]: AOT memory budget exceeded ({} > {} bytes)",
                static_alloc_bytes, mem_b
            );
            return Err(annotate_omni_error_with_sourcemap(&raw_err, &sm));
        }
        let i64_args: Vec<i64> = args
            .iter()
            .map(|a| match *a {
                OmniValue::Int(n) => n,
                OmniValue::Trit(t) => t as i64,
                _ => 0,
            })
            .collect();
        let wasm = compile_pure_u16_to_wasm(&u16_stream)
            .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
        let raw_i64 = execute_wasm_module_i64_with_budgets(&wasm, &i64_args, step_b, mem_b)
            .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
        let out_val = if entry_ret_ty_tok == TY_TRIT {
            OmniValue::Trit(raw_i64.clamp(-1, 1) as i8)
        } else {
            OmniValue::Int(raw_i64)
        };
        return Ok((out_val, "aot"));
    }

    let mut vm = OmniBinaryVm::new();
    if let Some(sb) = attr_steps {
        vm.step_limit = sb;
    }
    if let Some(mb) = attr_mem {
        vm.memory_budget_bytes = mb;
    }
    let val = vm
        .execute_function(&module, entry_fn_idx, args)
        .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
    Ok((val, "vm"))
}

/// Compiles, statically verifies all functions in the module, and executes an Omni surface program
/// (dispatching hot numeric/ternary/`@PACK5` paths through AOT first with explicit step and memory budgets).
pub fn execute_omni_surface_program(src: &str, args: Vec<OmniValue>) -> Result<OmniValue, String> {
    execute_omni_surface_aot_first(src, args).map(|(v, _)| v)
}

/// Compiles a surface `.omni` program via the Stage-1 self-hosting compiler (`omni_self_compiler.omni_bc`)
/// into a standalone 64-bit Linux x86-64 ELF64 binary.
pub fn compile_omni_surface_via_stage1_to_elf(
    src: &str,
    arg0: i64,
    arg1: i64,
    arg2: i64,
) -> Result<Vec<u8>, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            return Err(format!("{} (fn #{}: {:?})", first, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    if module.functions.len() != 1 {
        return Err(String::from("Stage-1 ELF64 emitter requires a single-function module"));
    }
    for ins in &module.functions[0].instructions {
        if !is_x86_64_aot_supported_opcode(ins.op_tok) {
            return Err(format!("Unsupported opcode 0x{:04X} in Stage-1 ELF64 emitter", ins.op_tok));
        }
    }
    let s1_stream = build_step7_stage1_compiler_stream();
    let s1_mod = decode_omni_token_stream(&s1_stream)?;
    let mut vm = OmniBinaryVm::new();
    let res = vm.execute_function(
        &s1_mod,
        0,
        vec![
            OmniValue::U16Array(u16_stream),
            OmniValue::Int(arg0),
            OmniValue::Int(arg1),
            OmniValue::Int(arg2),
        ],
    )?;
    match res {
        OmniValue::Bytes(elf) => Ok(elf),
        other => Err(format!("Stage-1 self-compiler returned non-Bytes: {:?}", other)),
    }
}

/// Compiles a surface `.omni` program directly to a standalone 64-bit Linux x86-64 ELF64 binary.
pub fn compile_omni_surface_to_x86_64_elf(
    src: &str,
    arg0: i64,
    arg1: i64,
    arg2: i64,
) -> Result<Vec<u8>, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    compile_pure_u16_to_x86_64_elf_with_args(&u16_stream, arg0, arg1, arg2)
}

/// Compiles a surface `.omni` program directly to a standalone 64-bit WebAssembly (`\0asm` v1) binary.
pub fn compile_omni_surface_to_wasm(src: &str) -> Result<Vec<u8>, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    compile_pure_u16_to_wasm(&u16_stream)
}

/// Compiles a surface `.omni` program to 64-bit WebAssembly and executes it in the pure-Rust 64-bit Wasm evaluator.
pub fn execute_omni_surface_wasm_i64(src: &str, args: &[i64]) -> Result<i64, String> {
    let wrapped = wrap_omni_surface_snippet(src);
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let step_b = attr_steps.unwrap_or(DEFAULT_WASM_STEP_LIMIT);
    let mem_b = attr_mem.unwrap_or(DEFAULT_WASM_MEMORY_BUDGET_BYTES);
    let u16_stream = compile_omni_surface_to_u16(&wrapped)?;
    let module = decode_omni_token_stream(&u16_stream)?;
    let mut static_alloc_bytes = 0usize;
    for func in &module.functions {
        for ins in &func.instructions {
            if ins.op_tok == 0x7F01 {
                if let Some(BinOperand::ImmInt(cap)) = ins.operands.first() {
                    if *cap > 0 {
                        static_alloc_bytes = static_alloc_bytes.saturating_add(*cap as usize);
                    }
                }
            }
        }
    }
    if static_alloc_bytes > mem_b {
        return Err(format!(
            "error[BUDGET_EXCEEDED]: AOT memory budget exceeded ({} > {} bytes)",
            static_alloc_bytes, mem_b
        ));
    }
    let wasm = compile_pure_u16_to_wasm(&u16_stream)?;
    execute_wasm_module_i64_with_budgets(&wasm, args, step_b, mem_b)
}

/// Regression: value-returning `if_pos`/`if_neg`/`match_trit` must use a single
/// contiguous SSA join slot. Branch temps in the first stream-order arm must not
/// punch a hole before the join dest is defined, and both arms must write that
/// same dest. Existing simple goldens (dest already written first) stay unchanged.
pub fn run_if_pos_join_slot_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let cases: &[(&str, &str, &[(i64, i64)], Option<usize>)] = &[
        (
            "arith_else_temps",
            "fn main(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { (x + 1) * 2 }; return r; }",
            &[(5, 200), (0, 2), (-3, -4)],
            None,
        ),
        (
            "let_in_else",
            "fn main(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { let y = x + 1; y }; return r; }",
            &[(5, 200), (0, 1), (-3, -2)],
            None,
        ),
        (
            "nested_complex_else",
            "fn main(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { 10 } else { let c1 = x <=> -5; if_pos c1 { 30 } else { 20 } }; return r; }",
            &[(5, 10), (0, 30), (-5, 20), (-6, 20)],
            None,
        ),
        (
            "match_trit_complex_neg",
            "fn main(x: i64) -> i64 { let c = x <=> 0; let r = match_trit c { neg => (x + 3) * 2, zero => 100, pos => x + 50 }; return r; }",
            &[(5, 55), (0, 100), (-3, 0)],
            None,
        ),
        (
            "simple_if_unchanged",
            "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }",
            &[(1, 200), (0, 100), (-1, 100)],
            Some(49),
        ),
    ];

    for &(name, src, ios, locked) in cases {
        let stream = compile_omni_surface_to_u16(src)
            .map_err(|e| format!("if_pos join-slot '{name}' failed to compile: {e}"))?;
        for rep in verify_u16_stream(&stream)? {
            if !rep.passed {
                return Err(format!(
                    "if_pos join-slot '{name}' failed binary verify (fn #{}): {:?}",
                    rep.fn_idx, rep.errors
                ));
            }
        }
        checks += 1;

        let asm = compile_omni_surface_to_asm(src)
            .map_err(|e| format!("if_pos join-slot '{name}' asm lower failed: {e}"))?;
        let via_asm = assemble_omni_text_to_u16(&asm)
            .map_err(|e| format!("if_pos join-slot '{name}' asm assemble failed: {e}"))?;
        if via_asm != stream {
            return Err(format!("if_pos join-slot '{name}': direct u16 != assembled asm"));
        }
        checks += 1;

        if let Some(n) = locked {
            if stream.len() != n {
                return Err(format!(
                    "if_pos join-slot '{name}' token count {} != locked {n} (join init must not change simple goldens)",
                    stream.len()
                ));
            }
            checks += 1;
        }

        let _ = compile_omni_surface_to_wasm(src)
            .map_err(|e| format!("if_pos join-slot '{name}' wasm compile failed: {e}"))?;
        let _ = compile_omni_surface_to_x86_64_elf(src, ios[0].0, 0, 0)
            .map_err(|e| format!("if_pos join-slot '{name}' x86-64 compile failed: {e}"))?;
        checks += 1;

        if name == "arith_else_temps" {
            let _ = compile_omni_surface_via_stage1_to_elf(src, ios[0].0, 0, 0)
                .map_err(|e| format!("if_pos join-slot '{name}' Stage-1 compile failed: {e}"))?;
            checks += 1;
        }

        for &(inp, want) in ios {
            let module = decode_omni_token_stream(&stream)?;
            let mut vm = OmniBinaryVm::new();
            let got = vm.execute_function(&module, 0, vec![OmniValue::Int(inp)])?;
            if got != OmniValue::Int(want) {
                return Err(format!("if_pos join-slot '{name}' VM x={inp} got {got:?} want {want}"));
            }
            let wasm = execute_omni_surface_wasm_i64(src, &[inp])?;
            if wasm != want {
                return Err(format!("if_pos join-slot '{name}' Wasm x={inp} got {wasm} want {want}"));
            }
            checks += 1;
        }
    }
    Ok(checks)
}

#[cfg(test)]
mod if_pos_join_slot_tests {
    #[test]
    fn if_pos_join_slot_regression() {
        let n = super::run_if_pos_join_slot_selftests().expect("if_pos join-slot regression");
        assert!(n >= 20, "expected at least 20 if_pos join-slot checks, got {n}");
        eprintln!("if_pos join-slot checks: {n}");
    }
}
