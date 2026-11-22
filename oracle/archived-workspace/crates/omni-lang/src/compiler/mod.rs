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
        // C4: 16 labels per function (token ids 0..15). Ids are handed out 1..15 first and the
        // 16th label gets id 0, so every function with <= 15 labels keeps its exact bytes (no
        // frozen hash moves). The 17th label is rejected.
        if self.next_lbl > 16 {
            return Err(format!(
                "error[BUDGET_EXCEEDED] line {}, col {}: Surface function exceeded 16 basic-block labels",
                line, col
            ));
        }
        let l = if self.next_lbl == 16 { 0 } else { self.next_lbl };
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
            "ld_u8" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'ld_u8' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("IGET {} -1 {}", op_args[0], op_args[1]))
            }
            "ld_u16" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'ld_u16' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("IGET {} -2 {}", op_args[0], op_args[1]))
            }
            "ld_u32" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'ld_u32' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("IGET {} -4 {}", op_args[0], op_args[1]))
            }
            "ld_u64" => {
                if op_args.len() != 2 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'ld_u64' expects 2 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("IGET {} -8 {}", op_args[0], op_args[1]))
            }
            "st_u16" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'st_u16' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -2 {} {}", op_args[0], op_args[1], op_args[2]))
            }
            "st_u32" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'st_u32' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -4 {} {}", op_args[0], op_args[1], op_args[2]))
            }
            "st_u64" => {
                if op_args.len() != 3 {
                    return Err(format!(
                        "error[ARITY_MISMATCH] line {}, col {}: Intrinsic 'st_u64' expects 3 arguments, got {}",
                        line, col, op_args.len()
                    ));
                }
                Ok(format!("ISET {} -9 {} {}", op_args[0], op_args[1], op_args[2]))
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
                if let SurfaceExpr::Call(cname, args, cl, cc) = expr {
                    if cname.eq_ignore_ascii_case("ret_slice") {
                        if args.len() != 3 {
                            return Err(format!(
                                "error[ARITY_MISMATCH] line {}, col {}: 'ret_slice' expects 3 arguments (buf, off, len)",
                                cl, cc
                            ));
                        }
                        let buf_op = self.eval_to_operand(&args[0])?;
                        let off_op = self.eval_to_operand(&args[1])?;
                        let len_op = self.eval_to_operand(&args[2])?;
                        self.cur_span = (*cl, *cc);
                        self.emit(format!("RET {} {} {}", buf_op, off_op, len_op))?;
                        return Ok(());
                    }
                }
                let op = self.eval_to_operand(expr)?;
                self.cur_span = (*line, *col);
                self.emit(format!("RET {}", op))?;
                Ok(())
            }
            SurfaceStmt::Expr(expr, line, col) => {
                if let SurfaceExpr::Call(cname, args, cl, cc) = expr {
                    let low = cname.to_ascii_lowercase();
                    if low == "ret_slice" {
                        if args.len() != 3 {
                            return Err(format!(
                                "error[ARITY_MISMATCH] line {}, col {}: 'ret_slice' expects 3 arguments (buf, off, len)",
                                cl, cc
                            ));
                        }
                        let buf_op = self.eval_to_operand(&args[0])?;
                        let off_op = self.eval_to_operand(&args[1])?;
                        let len_op = self.eval_to_operand(&args[2])?;
                        self.cur_span = (*cl, *cc);
                        self.emit(format!("RET {} {} {}", buf_op, off_op, len_op))?;
                        return Ok(());
                    } else if low == "br_pos" || low == "br_neg" {
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
            include_str!("../../../../../../std/prelude.omni")
        ),
        "std/math.omni" => Some(
            include_str!("../../../../../../std/math.omni")
        ),
        "std/trit.omni" => Some(
            include_str!("../../../../../../std/trit.omni")
        ),
        "std/http.omni" => Some(
            include_str!("../../../../../../std/http.omni")
        ),
        "std/pack5.omni" => Some(
            include_str!("../../../../../../std/pack5.omni")
        ),
        "std/decision.omni" => Some(
            include_str!("../../../../../../std/decision.omni")
        ),
        "std/resource.omni" => Some(
            include_str!("../../../../../../std/resource.omni")
        ),
        "pure/compiler.omni" | "spec/pure_self_host/omni_selfhost_compiler.omni" => Some(
            include_str!("../../../../../../pure/compiler.omni")
        ),
        "pure/verifier.omni" => Some(
            include_str!("../../../../../../pure/verifier.omni")
        ),
        "pure/host_runtime.omni" | "spec/pure_self_host/host_runtime.omni" => Some(
            include_str!("../../../../../../pure/host_runtime.omni")
        ),
        "pure/tooling.omni" => Some(
            include_str!("../../../../../../pure/tooling.omni")
        ),
        "pure/selfhost_suite.omni" => Some(
            include_str!("../../../../../../pure/selfhost_suite.omni")
        ),
        "pure/programs/ternary_neural_router.omni" => Some(
            include_str!("../../../../../../pure/programs/ternary_neural_router.omni")
        ),
        "pure/programs/arena_audit_pipeline.omni" => Some(
            include_str!("../../../../../../pure/programs/arena_audit_pipeline.omni")
        ),
        "pure/programs/policy_consensus_engine.omni" => Some(
            include_str!("../../../../../../pure/programs/policy_consensus_engine.omni")
        ),
        "pure/programs/kv_ledger_service.omni" => Some(
            include_str!("../../../../../../pure/programs/kv_ledger_service.omni")
        ),
        "pure/programs/compiler_self_audit.omni" => Some(
            include_str!("../../../../../../pure/programs/compiler_self_audit.omni")
        ),
        "pure/programs/pack5_decision_controller.omni" => Some(
            include_str!("../../../../../../pure/programs/pack5_decision_controller.omni")
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
        lockstep_shadow_verify("compile", &stream)?;
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
        lockstep_shadow_verify("compile", &stream)?;
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
    for rep in verify_u16_stream_lockstep("run", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
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
    for rep in verify_u16_stream_lockstep("run", &u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
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
    for rep in verify_u16_stream_lockstep("run", &u16_stream)? {
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
    stage1_reject_unsupported_shapes(&module.functions[0])?;
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

/// Host-side gate in front of the frozen Stage-1 compiler image: operand shapes and opcodes its
/// templates do not implement are rejected with a structured error instead of being mis-compiled
/// (the image itself is frozen, so this changes no Stage-1 bytes).
pub fn stage1_reject_unsupported_shapes(func: &BinFunction) -> Result<(), String> {
    for (ii, ins) in func.instructions.iter().enumerate() {
        if ins.op_tok == 0x7F0E && ins.operands.len() == 3 {
            return Err(format!(
                "error[UNSUPPORTED]: Stage-1 does not implement 3-op IGET (ld_u16/ld_u32/ld_u64) (ins {ii})"
            ));
        }
        if ins.op_tok == 0x7F0F
            && ins.operands.len() == 4
            && matches!(ins.operands[1], BinOperand::ImmInt(-2) | BinOperand::ImmInt(-4))
        {
            return Err(format!(
                "error[UNSUPPORTED]: Stage-1 does not implement 4-op ISET -2/-4 (st_u16/st_u32) (ins {ii})"
            ));
        }
        match ins.op_tok {
            0x7F05 => {
                return Err(format!(
                    "error[UNSUPPORTED]: Stage-1 does not implement DROP 0x7F05 (ins {ii})"
                ));
            }
            0x7F14 => {
                return Err(format!(
                    "error[UNSUPPORTED]: Stage-1 does not implement IMOD 0x7F14 safely (ins {ii})"
                ));
            }
            0x7F30 | 0x7F31 | 0x7F34 | 0x7F35 | 0x7F3F => {
                return Err(format!(
                    "error[UNSUPPORTED]: Stage-1 does not implement pack5/ternary-AI opcode 0x{:04X} (ins {ii})",
                    ins.op_tok
                ));
            }
            _ => {}
        }
    }
    Ok(())
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


/// Run a freshly emitted x86-64 ELF and decode the 8-byte little-endian stdout i64.
pub fn execute_native_elf_i64(elf: &[u8], args: &[i64]) -> Result<i64, String> {
    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let id = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "omni_native_{}_{}.elf",
            std::process::id(),
            id
        ));
        std::fs::write(&path, elf).map_err(|e| e.to_string())?;
        let mut perms = std::fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
        let arg_strs: Vec<String> = args.iter().map(|n| n.to_string()).collect();
        let out = std::process::Command::new(&path)
            .args(&arg_strs)
            .output()
            .map_err(|e| format!("failed to run native ELF: {e}"))?;
        let _ = std::fs::remove_file(&path);
        if out.status.code() == Some(OMNI_OOB_EXIT_STATUS)
            && out.stdout.is_empty()
            && out.stderr == format!("{}\n", OMNI_OOB_ERROR).as_bytes()
        {
            return Err(String::from(OMNI_OOB_ERROR));
        }
        if !out.status.success() || out.stdout.len() < 8 {
            return Err(format!(
                "native ELF failed: status {:?}, stdout {} bytes, stderr {}",
                out.status,
                out.stdout.len(),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(i64::from_le_bytes(out.stdout[0..8].try_into().unwrap()))
    }
    #[cfg(not(all(unix, target_arch = "x86_64")))]
    {
        let _ = (elf, args);
        Err(String::from("native ELF execution requires unix x86_64"))
    }
}

/// Vera P1 sibling of `byte_buf_golden`: runtime byte readback of an ISET buffer.
/// `byte_buf_golden` itself stays locked at 86 tokens and is not modified.
pub const BYTE_BUF_GOLDEN_SRC: &str = "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }";
pub const BYTE_BUF_GOLDEN_LOCKED_TOKENS: usize = 86;
pub const BYTE_READBACK_GOLDEN_SRC: &str = "fn byte_readback_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, x, 240); buf = iset_i32(buf, 0 - 2); let b0 = iget(buf, 0); let b1 = iget(buf, 1); let b2 = iget(buf, 2); let b3 = iget(buf, 3); let b6 = iget(buf, 6); let n = slen(buf); let r = ((((n * 256 + b6) * 256 + b3) * 256 + b2) * 256 + b1) * 256 + b0; return r; }";
pub const BYTE_READBACK_X: i64 = 7;
pub const BYTE_READBACK_EXPECTED: i64 = 8796075198218;

/// Last native x86 / Stage-1 Vera P1 results (None if not executed). x86-64 is
/// asserted: a 2-op IGET on an ANEW/ISET buffer register reads the ISET heap.
/// Stage-1 is RECORD-ONLY: its frozen IGET template still loads the u16 token
/// region, and teaching it heap readback would move the frozen Stage-1 hashes
/// (`0x677F57AE71D28FED` / `0xEAFC81F46B482FF8`). Do not read a Stage-1 value here as a pass.
pub fn last_byte_readback_native_results() -> (Option<i64>, Option<i64>) {
    (
        LAST_BYTE_READBACK_X86.with(|c| c.get()),
        LAST_BYTE_READBACK_STAGE1.with(|c| c.get()),
    )
}

std::thread_local! {
    static LAST_BYTE_READBACK_X86: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
    static LAST_BYTE_READBACK_STAGE1: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

/// 4-backend native run of Vera P1 byte readback. Asserts VM, Wasm, and native
/// x86-64 match the expected packed value (8796075198218), keeps `byte_buf_golden`
/// at 86 tokens, and executes the Stage-1 ELF natively as record-only (see above).
pub fn run_byte_readback_parity_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let buf_stream = compile_omni_surface_to_u16(BYTE_BUF_GOLDEN_SRC)
        .map_err(|e| format!("byte_buf_golden failed to compile: {e}"))?;
    if buf_stream.len() != BYTE_BUF_GOLDEN_LOCKED_TOKENS {
        return Err(format!(
            "byte_buf_golden token count {} != locked {}",
            buf_stream.len(),
            BYTE_BUF_GOLDEN_LOCKED_TOKENS
        ));
    }
    checks += 1;

    let stream = compile_omni_surface_to_u16(BYTE_READBACK_GOLDEN_SRC)
        .map_err(|e| format!("byte_readback_golden failed to compile: {e}"))?;
    let module = decode_omni_token_stream(&stream)?;
    let mut vm = OmniBinaryVm::new();
    let vm_got = vm.execute_function(&module, 0, vec![OmniValue::Int(BYTE_READBACK_X)])?;
    if vm_got != OmniValue::Int(BYTE_READBACK_EXPECTED) {
        return Err(format!(
            "Vera P1 VM x={BYTE_READBACK_X} got {vm_got:?} want {BYTE_READBACK_EXPECTED}"
        ));
    }
    checks += 1;

    let wasm_got = execute_omni_surface_wasm_i64(BYTE_READBACK_GOLDEN_SRC, &[BYTE_READBACK_X])?;
    if wasm_got != BYTE_READBACK_EXPECTED {
        return Err(format!(
            "Vera P1 Wasm x={BYTE_READBACK_X} got {wasm_got} want {BYTE_READBACK_EXPECTED}"
        ));
    }
    checks += 1;

    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        let elf = compile_omni_surface_to_x86_64_elf(
            BYTE_READBACK_GOLDEN_SRC,
            BYTE_READBACK_X,
            0,
            0,
        )?;
        let x86_got = execute_native_elf_i64(&elf, &[BYTE_READBACK_X])?;
        LAST_BYTE_READBACK_X86.with(|c| c.set(Some(x86_got)));
        if x86_got != BYTE_READBACK_EXPECTED {
            return Err(format!(
                "Vera P1 x86-64 x={BYTE_READBACK_X} got {x86_got} want {BYTE_READBACK_EXPECTED}"
            ));
        }
        checks += 1;

        let s1_elf = compile_omni_surface_via_stage1_to_elf(
            BYTE_READBACK_GOLDEN_SRC,
            BYTE_READBACK_X,
            0,
            0,
        )?;
        let s1_got = execute_native_elf_i64(&s1_elf, &[BYTE_READBACK_X])?;
        LAST_BYTE_READBACK_STAGE1.with(|c| c.set(Some(s1_got)));
        checks += 1;
        eprintln!(
            "Vera P1 x={BYTE_READBACK_X}: VM={BYTE_READBACK_EXPECTED} Wasm={wasm_got} x86={x86_got} Stage-1={s1_got} (Stage-1 record-only: frozen token-region IGET)"
        );
    }
    #[cfg(not(all(unix, target_arch = "x86_64")))]
    {
        eprintln!(
            "Vera P1 x={BYTE_READBACK_X}: VM={BYTE_READBACK_EXPECTED} Wasm={wasm_got} x86=skipped Stage-1=skipped"
        );
    }
    Ok(checks)
}

/// Vera P6 (`pack5_flow`): a branch-dependent all-immediate buffer must not be const-folded.
pub const BUF_FLOW_SRC: &str = "fn pack5_flow_golden(x: i64) -> i64 { let buf = anew(0); let c = x <=> 100; let b2 = if_pos c { iset(buf, 9) } else { iset(buf, 0 - 1, 44) }; let v0 = iget(b2, 0); let v1 = iget(b2, 1); let n = slen(b2); let r = n * 1000000 + v0 * 1000 + v1; return r; }";
/// x=7 -> else arm [255, 44] -> 2*10^6 + 255*1000 + 44; x=200 -> then arm [9] -> 10^6 + 9000 + 0.
pub const BUF_FLOW_RUNS: &[(i64, i64)] = &[(7, 2255044), (200, 1009000)];
/// Vera P7 (`two_bufs`): two ANEW buffers keep independent lengths: 3*10 + 1.
pub const TWO_BUFS_SRC: &str = "fn twob(x: i64) -> i64 { let a = anew(0); a = iset(a, 1, 2, 3); let b = anew(0); b = iset(b, 4); let la = slen(a); let lb = slen(b); let r = la * 10 + lb; return r; }";
pub const TWO_BUFS_EXPECTED: i64 = 31;
/// Value semantics on append: a=[1,2]; b=a++[3]; c=a++[9,x]. la=2 lb=3 lc=4, b[2]=3, c[2]=9,
/// c[3]=x, a[2]=0 (OOB) -> ((((2*10+3)*10+4)*100+3)*100+9)*1000 + x + 0.
pub const BUF_COPY_SRC: &str = "fn bufcopy(x: i64) -> i64 { let a = anew(0); a = iset(a, 1, 2); let b = iset(a, 3); let c = iset(a, 9, x); let la = slen(a); let lb = slen(b); let lc = slen(c); let r = ((((la * 10 + lb) * 10 + lc) * 100 + iget(b, 2)) * 100 + iget(c, 2)) * 1000 + iget(c, 3) + iget(a, 2) * 7; return r; }";
pub const BUF_COPY_X: i64 = 7;
pub const BUF_COPY_EXPECTED: i64 = 2340309007;

/// 13BB. Per-buffer storage: VM and Wasm must match the hand-derived values for P6, P7 and the
/// append-copy case; native x86-64 must match P6. x86-64 P7 / copy are RECORD-ONLY: the x86
/// emitter keeps one global r11 heap length, and changing ANEW/ISET/SLEN there would move the
/// frozen Stage-1 ELF hash `0xEAFC81F46B482FF8`. Stage-1 is not run here (token-region IGET).
pub fn run_buffer_flow_parity_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let vm_run = |src: &str, x: i64| -> Result<i64, String> {
        let stream = compile_omni_surface_to_u16(src)?;
        let module = decode_omni_token_stream(&stream)?;
        let mut vm = OmniBinaryVm::new();
        match vm.execute_function(&module, 0, vec![OmniValue::Int(x)])? {
            OmniValue::Int(v) => Ok(v),
            other => Err(format!("non-Int VM result {other:?}")),
        }
    };
    let mut cases: Vec<(&str, &str, i64, i64)> = BUF_FLOW_RUNS.iter().map(|&(x, want)| ("pack5_flow", BUF_FLOW_SRC, x, want)).collect();
    cases.push(("two_bufs", TWO_BUFS_SRC, 7, TWO_BUFS_EXPECTED));
    cases.push(("buf_copy", BUF_COPY_SRC, BUF_COPY_X, BUF_COPY_EXPECTED));
    for &(name, src, x, want) in &cases {
        let vm_got = vm_run(src, x)?;
        if vm_got != want {
            return Err(format!("{name} VM x={x} got {vm_got} want {want}"));
        }
        checks += 1;
        let wasm_got = execute_omni_surface_wasm_i64(src, &[x])?;
        if wasm_got != want {
            return Err(format!("{name} Wasm x={x} got {wasm_got} want {want}"));
        }
        checks += 1;
        let (run_got, path) = execute_omni_surface_aot_first(src, vec![OmniValue::Int(x)])?;
        if run_got != OmniValue::Int(want) {
            return Err(format!("{name} `omnic run` path ({path}) x={x} got {run_got:?} want {want}"));
        }
        checks += 1;
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            let elf = compile_omni_surface_to_x86_64_elf(src, x, 0, 0)?;
            let x86_got = execute_native_elf_i64(&elf, &[x])?;
            if name == "pack5_flow" {
                if x86_got != want {
                    return Err(format!("{name} x86-64 x={x} got {x86_got} want {want}"));
                }
                checks += 1;
            } else {
                eprintln!("{name} x={x}: VM={vm_got} Wasm={wasm_got} x86={x86_got} (x86 record-only: shared r11 heap)");
            }
        }
    }
    Ok(checks)
}

/// ISA-B golden: 3-op IGET (`ld_u16`/`ld_u32`/`ld_u64`, zero-extended LE) and 4-op ISET
/// patches (`st_u16`/`st_u32`; `st_u64` lowers to the existing -9 patch), all in bounds.
/// Bytes after the appends: [10..80] ++ i32(-2) ++ i64(x) (20 bytes). Hand-derived for x=7:
/// a=ld_u16@0 after st_u16(-300)=65236, b=ld_u32@4=305419896, c=ld_u32@8=0xFFFFFFFE,
/// d=ld_u64@12 (last 8-byte slot)=x, f=ld_u16@18 (last 2-byte slot)=x>>48&0xFFFF,
/// h=ld_u32@16 after st_u32(-1)=0xFFFFFFFF, n=20. r = a + 3b + 5c + 7d + 13f + 19h + 23n.
pub const MULTIBYTE_GOLDEN_SRC: &str = "fn multibyte_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30, 40, 50, 60, 70, 80); buf = iset_i32(buf, 0 - 2); buf = iset_i64(buf, x); buf = st_u16(buf, 0, 0 - 300); buf = st_u32(buf, 4, 305419896); let a = ld_u16(buf, 0); let b = ld_u32(buf, 4); let c = ld_u32(buf, 8); let d = ld_u64(buf, 12); let f = ld_u16(buf, 18); buf = st_u32(buf, 16, 0 - 1); let h = ld_u32(buf, 16); let n = slen(buf); let r = a + b * 3 + c * 5 + d * 7 + f * 13 + h * 19 + n * 23; return r; }";
/// x=7: 65236 + 916259688 + 21474836470 + 49 + 0 + 81604378605 + 460.
/// x=-1: d=-1 (u64::MAX as i64), f=65535: 103995540508 - 49 - 7 + 851955.
pub const MULTIBYTE_GOLDEN_RUNS: &[(i64, i64)] = &[(7, 103995540508), (-1, 103996392407)];

/// ISA-B strict bounds: every one of these is a deterministic runtime error[OUT_OF_BOUNDS] on
/// VM, Wasm and x86-64 (never read-as-0, never a silent no-op). Offsets come from `x` so no
/// backend can constant-fold them. 20-byte buffer unless noted.
pub const MULTIBYTE_OOB_SRCS: &[(&str, &str)] = &[
    ("ld_u16_tail", "fn oob(x: i64) -> i64 { let buf = anew(0); buf = iset_i64(buf, x); buf = iset_i64(buf, x); buf = iset_i32(buf, x); let o = x - x + 19; let v = ld_u16(buf, o); return v; }"),
    ("ld_u64_neg", "fn oob(x: i64) -> i64 { let buf = anew(0); buf = iset_i64(buf, x); buf = iset_i64(buf, x); buf = iset_i32(buf, x); let o = x - x - 1; let v = ld_u64(buf, o); return v; }"),
    ("ld_u32_overflow", "fn oob(x: i64) -> i64 { let buf = anew(0); buf = iset_i64(buf, x); buf = iset_i64(buf, x); buf = iset_i32(buf, x); let one = x - x + 1; let big = (one << 63) - 1; let v = ld_u32(buf, big); return v; }"),
    ("ld_u32_empty", "fn oob(x: i64) -> i64 { let buf = anew(0); let o = x - x; let v = ld_u32(buf, o); return v; }"),
    ("st_u16_tail", "fn oob(x: i64) -> i64 { let buf = anew(0); buf = iset_i64(buf, x); buf = iset_i64(buf, x); buf = iset_i32(buf, x); let o = x - x + 19; buf = st_u16(buf, o, 99); let n = slen(buf); return n; }"),
    ("st_u16_neg", "fn oob(x: i64) -> i64 { let buf = anew(0); buf = iset_i64(buf, x); let o = x - x - 1; buf = st_u16(buf, o, 99); let n = slen(buf); return n; }"),
    ("st_u32_overflow", "fn oob(x: i64) -> i64 { let buf = anew(0); buf = iset_i64(buf, x); let one = x - x + 1; let big = (one << 63) - 1; buf = st_u32(buf, big, 5); let n = slen(buf); return n; }"),
];

fn stream_has_isa_b_shapes(stream: &[u16]) -> Result<(usize, usize), String> {
    let module = decode_omni_token_stream(stream)?;
    let (mut iget3, mut iset4) = (0usize, 0usize);
    for func in &module.functions {
        for ins in &func.instructions {
            if ins.op_tok == 0x7F0E && ins.operands.len() == 3 {
                iget3 += 1;
            }
            if ins.op_tok == 0x7F0F
                && ins.operands.len() == 4
                && matches!(ins.operands[1], BinOperand::ImmInt(-2) | BinOperand::ImmInt(-4))
            {
                iset4 += 1;
            }
        }
    }
    Ok((iget3, iset4))
}

/// 13BC. ISA-B: VM, Wasm, the `omnic run` path and native x86-64 must all equal the hand-derived
/// in-bounds golden, and every MULTIBYTE_OOB_SRCS case must be a deterministic runtime
/// error[OUT_OF_BOUNDS] on all of them. Stage-1 must REJECT the new shapes up front with
/// error[UNSUPPORTED] (its frozen templates would mis-compile them; teaching it would move the
/// frozen Stage-1 hashes). Frozen streams must not contain the new shapes, and a bad 3-op IGET
/// mode is rejected.
pub fn run_isa_b_multibyte_parity_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    for (name, stream) in [
        ("byte_buf_golden", compile_omni_surface_to_u16(BYTE_BUF_GOLDEN_SRC)?),
        ("build_step7_stage1_compiler_stream", build_step7_stage1_compiler_stream()),
    ] {
        let (i3, s4) = stream_has_isa_b_shapes(&stream)?;
        if i3 != 0 || s4 != 0 {
            return Err(format!("frozen stream '{name}' contains ISA-B shapes: 3-op IGET={i3} 4-op ISET -2/-4={s4}"));
        }
        checks += 1;
    }
    let stream = compile_omni_surface_to_u16(MULTIBYTE_GOLDEN_SRC)?;
    let (i3, s4) = stream_has_isa_b_shapes(&stream)?;
    if i3 != 6 || s4 != 3 {
        return Err(format!("multibyte_golden lowering: 3-op IGET={i3} (want 6) 4-op ISET -2/-4={s4} (want 3)"));
    }
    checks += 1;
    for rep in verify_u16_stream(&stream)? {
        if !rep.passed {
            return Err(format!("multibyte_golden failed binary verify (fn #{}): {:?}", rep.fn_idx, rep.errors));
        }
    }
    checks += 1;
    let module = decode_omni_token_stream(&stream)?;
    for &(x, want) in MULTIBYTE_GOLDEN_RUNS {
        let mut vm = OmniBinaryVm::new();
        let vm_got = vm.execute_function(&module, 0, vec![OmniValue::Int(x)])?;
        if vm_got != OmniValue::Int(want) {
            return Err(format!("multibyte_golden VM x={x} got {vm_got:?} want {want}"));
        }
        checks += 1;
        let wasm_got = execute_omni_surface_wasm_i64(MULTIBYTE_GOLDEN_SRC, &[x])?;
        if wasm_got != want {
            return Err(format!("multibyte_golden Wasm x={x} got {wasm_got} want {want}"));
        }
        checks += 1;
        let (run_got, path) = execute_omni_surface_aot_first(MULTIBYTE_GOLDEN_SRC, vec![OmniValue::Int(x)])?;
        if run_got != OmniValue::Int(want) {
            return Err(format!("multibyte_golden run path ({path}) x={x} got {run_got:?} want {want}"));
        }
        checks += 1;
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            let elf = compile_omni_surface_to_x86_64_elf(MULTIBYTE_GOLDEN_SRC, x, 0, 0)?;
            let x86_got = execute_native_elf_i64(&elf, &[x])?;
            if x86_got != want {
                return Err(format!("multibyte_golden x86-64 x={x} got {x86_got} want {want}"));
            }
            checks += 1;
        }
        match compile_omni_surface_via_stage1_to_elf(MULTIBYTE_GOLDEN_SRC, x, 0, 0) {
            Err(e) if e.contains("error[UNSUPPORTED]") => checks += 1,
            Err(e) => return Err(format!("multibyte_golden Stage-1: wrong error {e}")),
            Ok(_) => return Err(String::from("multibyte_golden Stage-1 compiled ISA-B shapes; it must reject them")),
        }
    }
    // Strict bounds: out of bounds is error[OUT_OF_BOUNDS] at run time on every backend that
    // supports ISA-B, for both signs of x.
    for &(name, src) in MULTIBYTE_OOB_SRCS {
        let oob_stream = compile_omni_surface_to_u16(src)?;
        let oob_mod = decode_omni_token_stream(&oob_stream)?;
        for x in [3i64, -3] {
            let mut vm = OmniBinaryVm::new();
            #[allow(unused_mut)]
            let mut results: Vec<(&str, Result<i64, String>)> = vec![
                ("vm", vm.execute_function(&oob_mod, 0, vec![OmniValue::Int(x)]).map(|v| match v { OmniValue::Int(n) => n, _ => 0 })),
                ("wasm", execute_omni_surface_wasm_i64(src, &[x])),
                ("run", execute_omni_surface_aot_first(src, vec![OmniValue::Int(x)]).map(|_| 0)),
            ];
            #[cfg(all(unix, target_arch = "x86_64"))]
            results.push(("x86", compile_omni_surface_to_x86_64_elf(src, x, 0, 0).and_then(|elf| execute_native_elf_i64(&elf, &[x]))));
            for (be, res) in results {
                // Byte-identical canonical error on VM, Wasm and x86-64 (x86 maps only an exact
                // stderr line + exit status); the run path adds a source-map prefix.
                match res {
                    Err(e) if e == OMNI_OOB_ERROR => checks += 1,
                    Err(e) if be == "run" && e.starts_with("error[OUT_OF_BOUNDS]") && e.ends_with(&OMNI_OOB_ERROR["error[OUT_OF_BOUNDS]: ".len()..]) => checks += 1,
                    other => return Err(format!("ISA-B OOB '{name}' on {be} x={x}: expected {OMNI_OOB_ERROR:?}, got {other:?}")),
                }
            }
        }
    }
    // A 3-op IGET mode other than -2/-4/-8 is an error on VM, x86-64 and Wasm.
    let bad = assemble_omni_text_to_u16("PARAM $0 I64 SAFE\nRET_TY I64\n%0 = ANEW 0\n%0 = ISET %0 1 2 3 4\n%1 = IGET %0 -3 0\nRET %1\n")?;
    let bad_mod = decode_omni_token_stream(&bad)?;
    let mut vm = OmniBinaryVm::new();
    let vm_bad = vm.execute_function(&bad_mod, 0, vec![OmniValue::Int(1)]);
    let x86_bad = compile_pure_u16_to_x86_64_elf_with_args(&bad, 1, 0, 0);
    let wasm_bad = compile_pure_u16_to_wasm(&bad);
    for (be, res) in [("vm", vm_bad.map(|_| ())), ("x86", x86_bad.map(|_| ())), ("wasm", wasm_bad.map(|_| ()))] {
        match res {
            Err(e) if e.contains("error[BAD_TYPE]") => checks += 1,
            other => return Err(format!("bad 3-op IGET mode on {be}: expected error[BAD_TYPE], got {other:?}")),
        }
    }
    Ok(checks)
}

/// C4 label cap (16 per function). Sources match Vera's diff4 `lbl16_surface` / `lbl17_surface`.
/// 16 labels: 5 value if_pos (2 each) + 2 while (3 each). r = sum(if_pos c {k} else {10k}, k=1..5)
/// + 100*max(x,0) + max(x,0): x=5 -> 15 + 500 + 5 = 520; x=-5 -> 150.
pub const LBL16_SURFACE_SRC: &str = "fn lbl16(x: i64) -> i64 { let c = x <=> 0; let a1 = if_pos c { 1 } else { 10 }; let a2 = if_pos c { 2 } else { 20 }; let a3 = if_pos c { 3 } else { 30 }; let a4 = if_pos c { 4 } else { 40 }; let a5 = if_pos c { 5 } else { 50 }; let acc = 0; let i = 0; while i < x { acc += 100; i += 1; } let j = 0; while j < x { acc += 1; j += 1; } let r = a1 + a2 + a3 + a4 + a5 + acc; return r; }";
pub const LBL16_SURFACE_RUNS: &[(i64, i64)] = &[(5, 520), (-5, 150)];
/// 17 labels: 7 value if_pos + 1 while. Must be rejected.
pub const LBL17_SURFACE_SRC: &str = "fn lbl17(x: i64) -> i64 { let c = x <=> 0; let a1 = if_pos c { 1 } else { 10 }; let a2 = if_pos c { 2 } else { 20 }; let a3 = if_pos c { 3 } else { 30 }; let a4 = if_pos c { 4 } else { 40 }; let a5 = if_pos c { 5 } else { 50 }; let a6 = if_pos c { 6 } else { 60 }; let a7 = if_pos c { 7 } else { 70 }; let acc = 0; let i = 0; while i < x { acc += 100; i += 1; } let r = a1 + a2 + a3 + a4 + a5 + a6 + a7 + acc; return r; }";

/// 13BE. C4: a 16-label surface function compiles (16th label = id 0) and runs identically on
/// VM, Wasm, native x86-64 and Stage-1 (Stage-1 ELF == host ELF); a 17-label function is
/// rejected with error[BUDGET_EXCEEDED].
pub fn run_label_cap_16_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let stream = compile_omni_surface_to_u16(LBL16_SURFACE_SRC)?;
    let module = decode_omni_token_stream(&stream)?;
    let mut ids: Vec<u16> = Vec::new();
    for ins in &module.functions[0].instructions {
        if ins.op_tok == 0x7F40 {
            if let Some(l) = ins.operands.first().and_then(extract_label_id) {
                // Label operands carry the raw token 0x7EF0 + id.
                ids.push(l.wrapping_sub(0x7EF0));
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    if ids != (0u16..16).collect::<Vec<u16>>() {
        return Err(format!("lbl16: expected label ids 0..15, got {ids:?}"));
    }
    checks += 1;
    for &(x, want) in LBL16_SURFACE_RUNS {
        let mut vm = OmniBinaryVm::new();
        let vm_got = vm.execute_function(&module, 0, vec![OmniValue::Int(x)])?;
        if vm_got != OmniValue::Int(want) {
            return Err(format!("lbl16 VM x={x} got {vm_got:?} want {want}"));
        }
        checks += 1;
        let wasm_got = execute_omni_surface_wasm_i64(LBL16_SURFACE_SRC, &[x])?;
        if wasm_got != want {
            return Err(format!("lbl16 Wasm x={x} got {wasm_got} want {want}"));
        }
        checks += 1;
        #[cfg(all(unix, target_arch = "x86_64"))]
        {
            let elf = compile_omni_surface_to_x86_64_elf(LBL16_SURFACE_SRC, x, 0, 0)?;
            let x86_got = execute_native_elf_i64(&elf, &[x])?;
            if x86_got != want {
                return Err(format!("lbl16 x86-64 x={x} got {x86_got} want {want}"));
            }
            checks += 1;
            let s1 = compile_omni_surface_via_stage1_to_elf(LBL16_SURFACE_SRC, x, 0, 0)?;
            if s1 != elf {
                return Err(format!("lbl16 x={x}: Stage-1 ELF != host ELF"));
            }
            let s1_got = execute_native_elf_i64(&s1, &[x])?;
            if s1_got != want {
                return Err(format!("lbl16 Stage-1 x={x} got {s1_got} want {want}"));
            }
            checks += 1;
        }
    }
    match compile_omni_surface_to_u16(LBL17_SURFACE_SRC) {
        Err(e) if e.contains("error[BUDGET_EXCEEDED]") && e.contains("16 basic-block labels") => checks += 1,
        Err(e) => return Err(format!("lbl17: wrong rejection {e}")),
        Ok(_) => return Err(String::from("lbl17: a 17-label function compiled; it must be rejected")),
    }
    Ok(checks)
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

            #[cfg(all(unix, target_arch = "x86_64"))]
            {
                let elf = compile_omni_surface_to_x86_64_elf(src, inp, 0, 0)?;
                let native = execute_native_elf_i64(&elf, &[inp])?;
                if native != want {
                    return Err(format!(
                        "if_pos join-slot '{name}' x86-64 x={inp} got {native} want {want}"
                    ));
                }
                checks += 1;
            }
        }
    }
    Ok(checks)
}

// ==============================================================================
// M3-01. CANONICAL FRONTEND DUMPS (`fe-dump`) & THE M3 FRONTEND LOCK
// ==============================================================================
// Fixed binary formats the future Omni frontend (`pure/m3/`) is diffed against.
// Token dump: [M3_TOK_MAGIC, count_lo, count_hi] then per token
//   [kind, line_lo, line_hi, col_lo, col_hi, v0, v1, v2, v3, nbytes, bytes...]
//   (v0..v3 = little-endian i64 value: Int value or Trit value, else 0).
// Sourcemap dump: [M3_SM_MAGIC, n_lo, n_hi] then per entry
//   [off_lo, off_hi, line_lo, line_hi, col_lo, col_hi], then budget, default and pack5 spans.
// Errors are compared as `CLASS:line:col` (message text is not part of the lock).
// AST dump (0x4D41) and packet dump (0x4D50) formats are specified in spec/m3/FE_DUMP.md.
// The lock covers the closed suite modules (strict, as the suite compiles them), the
// verifier engine, the acceptance goldens on disk (minus the cwd-dependent
// `neg_circular_include`), `spec/m3/fixtures/q*.omni` (quirks Q1-Q8) and one
// `spec/m3/fixtures/neg_*.omni` per frontend error class.

pub const M3_FE_DUMP_VERSION: &str = "omni-m3-fe-dump-v1";
pub const M3_TOK_MAGIC: u16 = 0x4D33;
pub const M3_SM_MAGIC: u16 = 0x4D53;
pub const M3_TK_IDENT: u16 = 1;
pub const M3_TK_INT: u16 = 2;
pub const M3_TK_TRIT: u16 = 3;
pub const M3_TK_STR: u16 = 4;
pub const M3_TK_AT: u16 = 5;
pub const M3_TK_SYM: u16 = 6;
pub const M3_TK_OP: u16 = 7;
pub const M3_TK_HASH: u16 = 8;

pub const M3_FE_LOCK_TSV: &str = include_str!("../../../../../../spec/m3/corpus.tsv");

pub const M3_FE_LOOKUP_CORPUS: [&str; 18] = [
    "std/prelude.omni",
    "std/math.omni",
    "std/trit.omni",
    "std/http.omni",
    "std/pack5.omni",
    "std/decision.omni",
    "std/resource.omni",
    "pure/compiler.omni",
    "pure/verifier.omni",
    "pure/host_runtime.omni",
    "pure/tooling.omni",
    "pure/selfhost_suite.omni",
    "pure/programs/ternary_neural_router.omni",
    "pure/programs/arena_audit_pipeline.omni",
    "pure/programs/policy_consensus_engine.omni",
    "pure/programs/kv_ledger_service.omni",
    "pure/programs/compiler_self_audit.omni",
    "pure/programs/pack5_decision_controller.omni",
];

pub const M3_FE_FILE_CORPUS: &[(&str, &str)] = &[
    ("golden/arena_lifecycle_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/arena_lifecycle_golden.omni")),
    ("golden/asm_roundtrip.omni", include_str!("../../../../../../spec/acceptance/goldens/asm_roundtrip.omni")),
    ("golden/br_neg_arith_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/br_neg_arith_golden.omni")),
    ("golden/br_pos_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/br_pos_golden.omni")),
    ("golden/branch_loop_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/branch_loop_golden.omni")),
    ("golden/byte_buf_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/byte_buf_golden.omni")),
    ("golden/custom_non_seed_alu.omni", include_str!("../../../../../../spec/acceptance/goldens/custom_non_seed_alu.omni")),
    ("golden/decision_hook_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/decision_hook_golden.omni")),
    ("golden/iabs_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/iabs_golden.omni")),
    ("golden/idiv_zero.omni", include_str!("../../../../../../spec/acceptance/goldens/idiv_zero.omni")),
    ("golden/match_trit_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/match_trit_golden.omni")),
    ("golden/multi_fn_weighted_term.omni", include_str!("../../../../../../spec/acceptance/goldens/multi_fn_weighted_term.omni")),
    ("golden/neg_ambiguous_grammar.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_ambiguous_grammar.omni")),
    ("golden/neg_arity_mismatch.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_arity_mismatch.omni")),
    ("golden/neg_bad_type.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_bad_type.omni")),
    ("golden/neg_borrow_conflict.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_borrow_conflict.omni")),
    ("golden/neg_budget_exceeded.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_budget_exceeded.omni")),
    ("golden/neg_corrupt_pack5.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_corrupt_pack5.omni")),
    ("golden/neg_immut_param.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_immut_param.omni")),
    ("golden/neg_linear_leak.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_linear_leak.omni")),
    ("golden/neg_missing_cap.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_missing_cap.omni")),
    ("golden/neg_syntax_error.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_syntax_error.omni")),
    ("golden/neg_taint_on_ret.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_taint_on_ret.omni")),
    ("golden/neg_taint_sink.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_taint_sink.omni")),
    ("golden/neg_undef_var.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_undef_var.omni")),
    ("golden/neg_unknown_fn.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_unknown_fn.omni")),
    ("golden/neg_unresolved_label.omni", include_str!("../../../../../../spec/acceptance/goldens/neg_unresolved_label.omni")),
    ("golden/nested_branch_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/nested_branch_golden.omni")),
    ("golden/pack5_direct_aot.omni", include_str!("../../../../../../spec/acceptance/goldens/pack5_direct_aot.omni")),
    ("golden/pack5_trit_vec_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/pack5_trit_vec_golden.omni")),
    ("golden/route_db_query_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/route_db_query_golden.omni")),
    ("golden/route_kv_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/route_kv_golden.omni")),
    ("golden/seed0_risk.omni", include_str!("../../../../../../spec/acceptance/goldens/seed0_risk.omni")),
    ("golden/seed4_consensus.omni", include_str!("../../../../../../spec/acceptance/goldens/seed4_consensus.omni")),
    ("golden/seed6_clamp.omni", include_str!("../../../../../../spec/acceptance/goldens/seed6_clamp.omni")),
    ("golden/sel_mix.omni", include_str!("../../../../../../spec/acceptance/goldens/sel_mix.omni")),
    ("golden/shl_65.omni", include_str!("../../../../../../spec/acceptance/goldens/shl_65.omni")),
    ("golden/stdlib_macro_pipeline_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/stdlib_macro_pipeline_golden.omni")),
    ("golden/while_sum_even_golden.omni", include_str!("../../../../../../spec/acceptance/goldens/while_sum_even_golden.omni")),
    ("m3fix/q1_nonstrict_control.omni", include_str!("../../../../../../spec/m3/fixtures/q1_nonstrict_control.omni")),
    ("m3fix/q1_strict_substring.omni", include_str!("../../../../../../spec/m3/fixtures/q1_strict_substring.omni")),
    ("m3fix/q2_trit_text.omni", include_str!("../../../../../../spec/m3/fixtures/q2_trit_text.omni")),
    ("m3fix/q3_neg_fold.omni", include_str!("../../../../../../spec/m3/fixtures/q3_neg_fold.omni")),
    ("m3fix/q4_le_ge.omni", include_str!("../../../../../../spec/m3/fixtures/q4_le_ge.omni")),
    ("m3fix/q5_macro_after_def.omni", include_str!("../../../../../../spec/m3/fixtures/q5_macro_after_def.omni")),
    ("m3fix/q5_macro_order.omni", include_str!("../../../../../../spec/m3/fixtures/q5_macro_order.omni")),
    ("m3fix/q6_include_once.omni", include_str!("../../../../../../spec/m3/fixtures/q6_include_once.omni")),
    ("m3fix/q6_include_twice.omni", include_str!("../../../../../../spec/m3/fixtures/q6_include_twice.omni")),
    ("m3fix/q7_fn_ids.omni", include_str!("../../../../../../spec/m3/fixtures/q7_fn_ids.omni")),
    ("m3fix/q8a_ssa_limit.omni", include_str!("../../../../../../spec/m3/fixtures/q8a_ssa_limit.omni")),
    ("m3fix/q8b_label_limit.omni", include_str!("../../../../../../spec/m3/fixtures/q8b_label_limit.omni")),
    ("m3fix/q8c_label_ok.omni", include_str!("../../../../../../spec/m3/fixtures/q8c_label_ok.omni")),
    ("m3fix/q8d_label_15.omni", include_str!("../../../../../../spec/m3/fixtures/q8d_label_15.omni")),
    ("m3fix/q8e_label_17.omni", include_str!("../../../../../../spec/m3/fixtures/q8e_label_17.omni")),
    ("m3fix/neg_ambiguous_grammar.omni", include_str!("../../../../../../spec/m3/fixtures/neg_ambiguous_grammar.omni")),
    ("m3fix/neg_arity_mismatch.omni", include_str!("../../../../../../spec/m3/fixtures/neg_arity_mismatch.omni")),
    ("m3fix/neg_bad_type.omni", include_str!("../../../../../../spec/m3/fixtures/neg_bad_type.omni")),
    ("m3fix/neg_budget_exceeded.omni", include_str!("../../../../../../spec/m3/fixtures/neg_budget_exceeded.omni")),
    ("m3fix/neg_circular_include.omni", include_str!("../../../../../../spec/m3/fixtures/neg_circular_include.omni")),
    ("m3fix/neg_immut_param.omni", include_str!("../../../../../../spec/m3/fixtures/neg_immut_param.omni")),
    ("m3fix/neg_missing_cap.omni", include_str!("../../../../../../spec/m3/fixtures/neg_missing_cap.omni")),
    ("m3fix/neg_syntax_error.omni", include_str!("../../../../../../spec/m3/fixtures/neg_syntax_error.omni")),
    ("m3fix/neg_undef_var.omni", include_str!("../../../../../../spec/m3/fixtures/neg_undef_var.omni")),
    ("m3fix/neg_unknown_fn.omni", include_str!("../../../../../../spec/m3/fixtures/neg_unknown_fn.omni")),
    ("m3fix/neg_unresolved_label.omni", include_str!("../../../../../../spec/m3/fixtures/neg_unresolved_label.omni")),
];

fn m3_push_u32(out: &mut Vec<u16>, v: usize) -> Result<(), String> {
    if v > u32::MAX as usize {
        return Err(format!("fe-dump value {} exceeds u32", v));
    }
    out.push((v & 0xFFFF) as u16);
    out.push(((v >> 16) & 0xFFFF) as u16);
    Ok(())
}

/// Canonical token dump of `tokenize_omni_surface_inner` (same strictness rule as the compiler).
pub fn m3_dump_tokens(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    let effective_strict = strict || src.contains("#[strict]");
    let toks = tokenize_omni_surface_inner(src, effective_strict)?;
    let mut out = vec![M3_TOK_MAGIC];
    m3_push_u32(&mut out, toks.len())?;
    for t in &toks {
        let (kind, val, text): (u16, i64, &str) = match &t.tok {
            SurfaceTok::Ident(s) => (M3_TK_IDENT, 0, s.as_str()),
            SurfaceTok::Int(v) => (M3_TK_INT, *v, ""),
            SurfaceTok::Trit(s, v) => (M3_TK_TRIT, *v as i64, s.as_str()),
            SurfaceTok::StrLit(s) => (M3_TK_STR, 0, s.as_str()),
            SurfaceTok::AtIdent(s) => (M3_TK_AT, 0, s.as_str()),
            SurfaceTok::Sym(s) => (M3_TK_SYM, 0, s.as_str()),
            SurfaceTok::Op(s) => (M3_TK_OP, 0, s.as_str()),
            SurfaceTok::Hash => (M3_TK_HASH, 0, ""),
        };
        out.push(kind);
        m3_push_u32(&mut out, t.line)?;
        m3_push_u32(&mut out, t.col)?;
        let uv = val as u64;
        for k in 0..4 {
            out.push(((uv >> (16 * k)) & 0xFFFF) as u16);
        }
        let bytes = text.as_bytes();
        if bytes.len() > 0xFFFF {
            return Err(String::from("fe-dump token text longer than 65535 bytes"));
        }
        out.push(bytes.len() as u16);
        for b in bytes {
            out.push(*b as u16);
        }
    }
    Ok(out)
}

/// Canonical dump of `compile_omni_surface_to_u16_with_sourcemap`: (u16 stream, sourcemap words).
pub fn m3_dump_stream(src: &str, strict: bool) -> Result<(Vec<u16>, Vec<u16>), String> {
    let (stream, sm) = compile_omni_surface_to_u16_with_sourcemap(src, strict)?;
    let mut smw = vec![M3_SM_MAGIC];
    m3_push_u32(&mut smw, sm.offset_to_span.len())?;
    for (off, (l, c)) in &sm.offset_to_span {
        m3_push_u32(&mut smw, *off)?;
        m3_push_u32(&mut smw, *l)?;
        m3_push_u32(&mut smw, *c)?;
    }
    m3_push_u32(&mut smw, sm.budget_span.0)?;
    m3_push_u32(&mut smw, sm.budget_span.1)?;
    m3_push_u32(&mut smw, sm.default_span.0)?;
    m3_push_u32(&mut smw, sm.default_span.1)?;
    match sm.pack5_span {
        Some((l, c)) => {
            smw.push(1);
            m3_push_u32(&mut smw, l)?;
            m3_push_u32(&mut smw, c)?;
        }
        None => smw.push(0),
    }
    Ok((stream, smw))
}

/// `error[CLASS] line L, col C: ...` -> `CLASS:L:C` (anything else -> `OTHER`).
pub fn m3_error_key(e: &str) -> String {
    let class = match (e.find("error["), e.find(']')) {
        (Some(a), Some(b)) if b > a + 6 => &e[a + 6..b],
        _ => return String::from("OTHER"),
    };
    let num_after = |pat: &str| -> Option<usize> {
        let i = e.find(pat)? + pat.len();
        let digits: String = e[i..].chars().take_while(|ch| ch.is_ascii_digit()).collect();
        digits.parse::<usize>().ok()
    };
    match (num_after("line "), num_after("col ")) {
        (Some(l), Some(c)) => format!("{}:{}:{}", class, l, c),
        _ => format!("{}:?:?", class),
    }
}

fn m3_words_fnv(w: &[u16]) -> String {
    format!("{}:0x{:016X}", w.len(), fnv1a_64_hash(&encode_u16_stream_to_bytes(w)))
}

/// The token dump with every line/col zeroed: equal iff kinds, values and texts are equal.
pub fn m3_strip_token_spans(dump: &[u16]) -> Vec<u16> {
    let mut out = dump.to_vec();
    let mut i = 3usize;
    while i + 10 <= out.len() {
        for k in 1..5 {
            out[i + k] = 0;
        }
        let n = out[i + 9] as usize;
        i += 10 + n;
    }
    out
}

pub const M3_AST_MAGIC: u16 = 0x4D41;
pub const M3_PK_MAGIC: u16 = 0x4D50;

fn m3_push_str(out: &mut Vec<u16>, s: &str) -> Result<(), String> {
    let b = s.as_bytes();
    if b.len() > 0xFFFF {
        return Err(String::from("fe-dump string longer than 65535 bytes"));
    }
    out.push(b.len() as u16);
    for x in b {
        out.push(*x as u16);
    }
    Ok(())
}

fn m3_push_i64(out: &mut Vec<u16>, v: i64) {
    let uv = v as u64;
    for k in 0..4 {
        out.push(((uv >> (16 * k)) & 0xFFFF) as u16);
    }
}

fn m3_push_span(out: &mut Vec<u16>, l: usize, c: usize) -> Result<(), String> {
    m3_push_u32(out, l)?;
    m3_push_u32(out, c)
}

fn m3_ast_stmts(out: &mut Vec<u16>, body: &[SurfaceStmt]) -> Result<(), String> {
    m3_push_u32(out, body.len())?;
    for s in body {
        m3_ast_stmt(out, s)?;
    }
    Ok(())
}

// Rust-side serializer only (host code); recursion here is fine, the no-recursion rule is for core .omni.
fn m3_ast_stmt(out: &mut Vec<u16>, s: &SurfaceStmt) -> Result<(), String> {
    match s {
        SurfaceStmt::Let(name, ty, e, l, c) => {
            out.push(0x10);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, name)?;
            match ty {
                Some((t, tl, tc)) => {
                    out.push(1);
                    m3_push_str(out, t)?;
                    m3_push_span(out, *tl, *tc)?;
                }
                None => out.push(0),
            }
            m3_ast_expr(out, e)
        }
        SurfaceStmt::Assign(name, e, l, c) => {
            out.push(0x11);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, name)?;
            m3_ast_expr(out, e)
        }
        SurfaceStmt::DoWhile(body, cond, l, c) => {
            out.push(0x12);
            m3_push_span(out, *l, *c)?;
            m3_ast_stmts(out, body)?;
            m3_ast_expr(out, cond)
        }
        SurfaceStmt::While(cond, body, l, c) => {
            out.push(0x13);
            m3_push_span(out, *l, *c)?;
            m3_ast_expr(out, cond)?;
            m3_ast_stmts(out, body)
        }
        SurfaceStmt::Label(id, l, c) => {
            out.push(0x14);
            m3_push_span(out, *l, *c)?;
            out.push(*id);
            Ok(())
        }
        SurfaceStmt::Goto(id, l, c) => {
            out.push(0x15);
            m3_push_span(out, *l, *c)?;
            out.push(*id);
            Ok(())
        }
        SurfaceStmt::Ret(e, l, c) => {
            out.push(0x16);
            m3_push_span(out, *l, *c)?;
            m3_ast_expr(out, e)
        }
        SurfaceStmt::Expr(e, l, c) => {
            out.push(0x17);
            m3_push_span(out, *l, *c)?;
            m3_ast_expr(out, e)
        }
    }
}

fn m3_ast_expr(out: &mut Vec<u16>, e: &SurfaceExpr) -> Result<(), String> {
    match e {
        SurfaceExpr::Int(v, l, c) => {
            out.push(0x20);
            m3_push_span(out, *l, *c)?;
            m3_push_i64(out, *v);
            Ok(())
        }
        SurfaceExpr::Trit(s, v, l, c) => {
            out.push(0x21);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, s)?;
            out.push(*v as i16 as u16);
            Ok(())
        }
        SurfaceExpr::StrLit(s, l, c) => {
            out.push(0x22);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, s)
        }
        SurfaceExpr::TritVecLit(v, l, c) => {
            out.push(0x23);
            m3_push_span(out, *l, *c)?;
            m3_push_u32(out, v.len())?;
            for t in v {
                out.push(*t as i16 as u16);
            }
            Ok(())
        }
        SurfaceExpr::Var(s, l, c) => {
            out.push(0x24);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, s)
        }
        SurfaceExpr::UnOp(op, a, l, c) => {
            out.push(0x25);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, op)?;
            m3_ast_expr(out, a)
        }
        SurfaceExpr::BinOp(op, a, b, l, c) => {
            out.push(0x26);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, op)?;
            m3_ast_expr(out, a)?;
            m3_ast_expr(out, b)
        }
        SurfaceExpr::Call(name, args, l, c) => {
            out.push(0x27);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, name)?;
            m3_push_u32(out, args.len())?;
            for a in args {
                m3_ast_expr(out, a)?;
            }
            Ok(())
        }
        SurfaceExpr::IfBranch(kind, cond, then_b, else_b, l, c) => {
            out.push(0x28);
            m3_push_span(out, *l, *c)?;
            m3_push_str(out, kind)?;
            m3_ast_expr(out, cond)?;
            m3_ast_stmts(out, then_b)?;
            m3_ast_stmts(out, else_b)
        }
        SurfaceExpr::MatchTrit(scrut, neg, zero, pos, l, c) => {
            out.push(0x29);
            m3_push_span(out, *l, *c)?;
            m3_ast_expr(out, scrut)?;
            m3_ast_stmts(out, neg)?;
            m3_ast_stmts(out, zero)?;
            m3_ast_stmts(out, pos)
        }
    }
}

/// Lex + parse exactly as `compile_omni_surface_to_u16_with_sourcemap` does (including its
/// textual include expansion and re-parse), returning the parsed fns.
fn m3_parse_like_compile(src: &str, strict: bool) -> Result<(bool, Vec<SurfaceFnDef>), String> {
    let effective_strict = strict || src.contains("#[strict]");
    let tokens = tokenize_omni_surface_inner(src, effective_strict)?;
    let mut parser = SurfaceParser::new_with_strict(tokens, effective_strict);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return m3_parse_like_compile(&expanded, effective_strict);
    }
    Ok((effective_strict, fns))
}

/// Canonical preorder AST dump of `parse_module` (after macro substitution, as the parser does it).
pub fn m3_dump_ast(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    let (_, fns) = m3_parse_like_compile(src, strict)?;
    let mut out = vec![M3_AST_MAGIC];
    m3_push_u32(&mut out, fns.len())?;
    for f in &fns {
        out.push(0xF0);
        m3_push_span(&mut out, f.fn_line, f.fn_col)?;
        m3_push_str(&mut out, &f.name)?;
        match f.fn_id_attr {
            Some(id) => {
                out.push(1);
                out.push(id);
            }
            None => out.push(0),
        }
        out.push(f.cap_mask);
        match f.step_budget_attr {
            Some(v) => {
                out.push(1);
                m3_push_i64(&mut out, v as i64);
            }
            None => out.push(0),
        }
        match f.mem_budget_attr {
            Some(v) => {
                out.push(1);
                m3_push_i64(&mut out, v as i64);
            }
            None => out.push(0),
        }
        match f.budget_span {
            Some((l, c)) => {
                out.push(1);
                m3_push_span(&mut out, l, c)?;
            }
            None => out.push(0),
        }
        out.push(f.strict_attr as u16);
        out.push(f.is_route as u16);
        for opt in [&f.route_method, &f.route_path] {
            match opt {
                Some(s) => {
                    out.push(1);
                    m3_push_str(&mut out, s)?;
                }
                None => out.push(0),
            }
        }
        out.push(f.params.len() as u16);
        for p in &f.params {
            m3_push_str(&mut out, &p.name)?;
            m3_push_str(&mut out, &p.ty)?;
            out.push(p.untrusted as u16);
            m3_push_span(&mut out, p.line, p.col)?;
        }
        m3_push_str(&mut out, &f.ret_ty)?;
        m3_push_span(&mut out, f.ret_line, f.ret_col)?;
        m3_ast_stmts(&mut out, &f.body)?;
    }
    Ok(out)
}

/// Canonical per-function packet dump: the u16 packets `SurfaceLowerCtx` emits, each with its span.
/// Mirrors the front half of `compile_omni_surface_to_u16_with_sourcemap` (no lockstep) and then
/// re-packs the packets and checks the result equals the real compiler's stream word for word.
pub fn m3_dump_packets(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    let (effective_strict, fns) = m3_parse_like_compile(src, strict)?;
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
    let mut out = vec![M3_PK_MAGIC];
    m3_push_u32(&mut out, fns.len())?;
    let mut repack: Vec<u16> = Vec::new();
    if multi_fn {
        repack.push(TOK_MOD_BEGIN);
    }
    for (idx, f) in fns.iter().enumerate() {
        let fid = assigned_ids[idx];
        let fn_strict = effective_strict || f.strict_attr;
        let mut params = Vec::with_capacity(f.params.len());
        for (p_idx, p) in f.params.iter().enumerate() {
            let ty_str = map_omni_surface_type_inner(&p.ty, p.line, p.col, fn_strict)?;
            params.push((PAR_BASE + (p_idx as u16), parse_asm_ty_tok(ty_str), pack_tc(p.untrusted, CAP_ID_PURE)));
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
        out.push(fid);
        out.push(params.len() as u16);
        for (pt, pty, ptc) in &params {
            out.push(*pt);
            out.push(*pty);
            out.push(*ptc);
        }
        out.push(ret_ty_tok);
        out.push(f.cap_mask);
        m3_push_u32(&mut out, ctx.packets.len())?;
        for (pkt, sp) in ctx.packets.iter().zip(ctx.pkt_spans.iter()) {
            m3_push_span(&mut out, sp.0, sp.1)?;
            if pkt.len() > 0xFFFF {
                return Err(String::from("fe-dump packet longer than 65535 words"));
            }
            out.push(pkt.len() as u16);
            out.extend_from_slice(pkt);
        }
        if multi_fn {
            repack.push(TOK_FN_DEF);
            repack.push(fid);
            repack.push(params.len() as u16);
            for (pt, pty, ptc) in &params {
                repack.push(*pt);
                repack.push(*pty);
                repack.push(*ptc);
            }
            repack.push(ret_ty_tok);
            repack.push(f.cap_mask);
            repack.push(ctx.packets.len() as u16);
            for pkt in &ctx.packets {
                repack.extend_from_slice(pkt);
            }
        } else {
            repack = pack_fn_module(&[], fid, &params, ret_ty_tok, f.cap_mask, &ctx.packets);
        }
    }
    if multi_fn {
        repack.push(TOK_MOD_END);
    }
    if !fns.is_empty() {
        if let Ok((stream, _)) = compile_omni_surface_to_u16_with_sourcemap(src, strict) {
            if stream != repack {
                return Err(String::from("error[FE_DUMP_DIVERGENCE] line 1, col 1: packet dump does not re-pack to the compiler's stream"));
            }
        }
    }
    Ok(out)
}

/// One lock row: `name<TAB>strict<TAB>tok=..<TAB>ast=..<TAB>pk=..<TAB>out=..`.
pub fn m3_fe_lock_row(name: &str, src: &str, strict: bool) -> String {
    let tok = match m3_dump_tokens(src, strict) {
        Ok(t) => format!("tok={}:tokv={}", m3_words_fnv(&t), m3_words_fnv(&m3_strip_token_spans(&t))),
        Err(e) => format!("tok=ERR:{}", m3_error_key(&e)),
    };
    let ast = match m3_dump_ast(src, strict) {
        Ok(a) => format!("ast={}", m3_words_fnv(&a)),
        Err(e) => format!("ast=ERR:{}", m3_error_key(&e)),
    };
    let pk = match m3_dump_packets(src, strict) {
        Ok(p) => format!("pk={}", m3_words_fnv(&p)),
        Err(e) => format!("pk=ERR:{}", m3_error_key(&e)),
    };
    let out = match m3_dump_stream(src, strict) {
        Ok((s, sm)) => format!("out={}:sm={}", m3_words_fnv(&s), m3_words_fnv(&sm)),
        Err(e) => format!("out=ERR:{}", m3_error_key(&e)),
    };
    format!("{}\t{}\t{}\t{}\t{}\t{}", name, if strict { 1 } else { 0 }, tok, ast, pk, out)
}

/// The circular-include fixture names a repo-root-relative path, so the lock resolves it through
/// the resolver's in-memory VFS instead of the process cwd (same result as compiling it from the
/// repo root, without depending on where the test runs).
pub fn m3_fe_lock_row_circular(name: &str, vfs_path: &str, src: &str) -> String {
    let mut vfs = BTreeMap::new();
    vfs.insert(String::from(vfs_path), String::from(src));
    let inc = match resolve_omni_surface_includes(src, None, Some(&vfs)) {
        Ok(x) => format!("inc=OK:{}", x.len()),
        Err(e) => format!("inc=ERR:{}", m3_error_key(&e)),
    };
    format!("{}\t0\t{}", name, inc)
}

/// One negative fixture per frontend error class: (class, fixture name). Each must fail the
/// stream compile with exactly that class.
pub const M3_NEG_CLASS_FIXTURES: [(&str, &str); 11] = [
    ("SYNTAX_ERROR", "m3fix/neg_syntax_error.omni"),
    ("ARITY_MISMATCH", "m3fix/neg_arity_mismatch.omni"),
    ("UNRESOLVED_LABEL", "m3fix/neg_unresolved_label.omni"),
    ("BAD_TYPE", "m3fix/neg_bad_type.omni"),
    ("AMBIGUOUS_GRAMMAR", "m3fix/neg_ambiguous_grammar.omni"),
    ("UNDEF_VAR", "m3fix/neg_undef_var.omni"),
    ("BUDGET_EXCEEDED", "m3fix/neg_budget_exceeded.omni"),
    ("UNKNOWN_FN", "m3fix/neg_unknown_fn.omni"),
    ("MISSING_CAP", "m3fix/neg_missing_cap.omni"),
    ("IMMUT_PARAM", "m3fix/neg_immut_param.omni"),
    ("CIRCULAR_INCLUDE", "m3fix/neg_circular_include.omni"),
];

/// Every lock row, in manifest order.
pub fn m3_fe_lock_rows() -> Result<Vec<String>, String> {
    let mut rows = Vec::new();
    for key in M3_FE_LOOKUP_CORPUS.iter() {
        let src = lookup_omni_stdlib_module(key)
            .ok_or_else(|| format!("M3 lock: missing built-in module '{}'", key))?;
        rows.push(m3_fe_lock_row(&format!("lookup/{}", key), src, true));
    }
    rows.push(m3_fe_lock_row("engine/pure/verifier_engine.omni", crate::verifier::OMNI_VERIFIER_ENGINE_SURFACE, false));
    for (name, src) in M3_FE_FILE_CORPUS.iter() {
        if *name == "m3fix/neg_circular_include.omni" {
            rows.push(m3_fe_lock_row_circular(name, "spec/m3/fixtures/neg_circular_include.omni", src));
        } else {
            rows.push(m3_fe_lock_row(name, src, false));
        }
    }
    Ok(rows)
}

/// M3-01 lock: the Rust frontend's canonical dumps must equal `spec/m3/corpus.tsv` row for row.
pub fn run_m3_fe_lock_selftests() -> Result<usize, String> {
    let rows = m3_fe_lock_rows()?;
    let want: Vec<&str> = M3_FE_LOCK_TSV
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .collect();
    if want.len() != rows.len() {
        return Err(format!("M3 frontend lock: {} rows computed, {} locked", rows.len(), want.len()));
    }
    for (got, exp) in rows.iter().zip(want.iter()) {
        if got.as_str() != *exp {
            return Err(format!("M3 frontend lock mismatch:\n  got  {}\n  want {}", got, exp));
        }
    }
    let mut checks = rows.len();
    for (class, fx) in M3_NEG_CLASS_FIXTURES.iter() {
        let row = rows
            .iter()
            .find(|r| r.split('\t').next() == Some(*fx))
            .ok_or_else(|| format!("M3 negative fixture '{}' missing from the lock", fx))?;
        let want_out = format!("out=ERR:{}:", class);
        let want_inc = format!("inc=ERR:{}:", class);
        if !row.contains(&want_out) && !row.contains(&want_inc) {
            return Err(format!("M3 negative fixture '{}' does not fail with {}: {}", fx, class, row));
        }
        checks += 1;
    }
    Ok(checks)
}

/// M3-01 canonical dumper.
///   fe-dump lock                                         (prints the spec/m3/corpus.tsv rows)
///   fe-dump --tokens|--ast|--packets|--stream <file.omni> [out.bin] [--strict]
/// `tokens`/`ast`/`packets`/`stream` without dashes are accepted too. With `out.bin`, the dump is
/// written as little-endian u16 words; `--stream` also writes `<out.bin>.sm` (sourcemap dump).
/// Includes resolve as an in-memory compile does (built-in modules, then the cwd).
pub fn run_m3_fe_dump_cli(argv: &[String]) -> i32 {
    let strict = argv.iter().any(|a| a == "--strict");
    let rest: Vec<&str> = argv.iter().map(|s| s.as_str()).filter(|a| *a != "--strict").collect();
    let usage = || {
        eprintln!("usage: fe-dump lock | fe-dump --tokens|--ast|--packets|--stream <file.omni> [out.bin] [--strict]");
        2
    };
    let mode = match rest.first() {
        Some(m) => m.trim_start_matches('-'),
        None => return usage(),
    };
    if mode == "lock" {
        return match m3_fe_lock_rows() {
            Ok(rows) => {
                println!("# {} lock rows: name, strict, tok=words:fnv1a64:tokv=span-free words:fnv1a64, ast=words:fnv1a64, pk=words:fnv1a64, out=words:fnv1a64:sm=words:fnv1a64 (any column may be ERR:CLASS:line:col; inc= rows go through the include resolver)", M3_FE_DUMP_VERSION);
                for r in rows {
                    println!("{}", r);
                }
                0
            }
            Err(e) => {
                eprintln!("[FE-DUMP FAIL] {}", e);
                1
            }
        };
    }
    if !matches!(mode, "tokens" | "ast" | "packets" | "stream") || rest.len() < 2 {
        return usage();
    }
    let src = match fs::read_to_string(rest[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[FE-DUMP FAIL] cannot read '{}': {}", rest[1], e);
            return 1;
        }
    };
    let res = match mode {
        "tokens" => m3_dump_tokens(&src, strict).map(|t| (t, None)),
        "ast" => m3_dump_ast(&src, strict).map(|t| (t, None)),
        "packets" => m3_dump_packets(&src, strict).map(|t| (t, None)),
        _ => m3_dump_stream(&src, strict).map(|(s, sm)| (s, Some(sm))),
    };
    match res {
        Ok((words, sm)) => {
            let mut msg = format!("[FE-DUMP] {} {} words={}", mode, rest[1], m3_words_fnv(&words));
            if let Some(smw) = &sm {
                msg.push_str(&format!(" sm={}", m3_words_fnv(smw)));
            }
            if let Some(outp) = rest.get(2) {
                if let Err(e) = fs::write(outp, encode_u16_stream_to_bytes(&words)) {
                    eprintln!("[FE-DUMP FAIL] write: {}", e);
                    return 1;
                }
                if let Some(smw) = sm {
                    if let Err(e) = fs::write(format!("{}.sm", outp), encode_u16_stream_to_bytes(&smw)) {
                        eprintln!("[FE-DUMP FAIL] write: {}", e);
                        return 1;
                    }
                }
            }
            println!("{}", msg);
            0
        }
        Err(e) => {
            println!("[FE-DUMP] {} {} ERR:{}", mode, rest[1], m3_error_key(&e));
            1
        }
    }
}

// ==============================================================================
// M3-02. GATE FOR `pure/m3/`: no recursion, label/SSA budgets, call depth <= 64
// ==============================================================================
// The plan points `m2-lint` (M2-03) and the M2-08 module/call-depth check at `pure/m3/*.omni`.
// Neither tool is on this base (`d2d6c13`): `m2-lint` exists only as uncommitted WIP, and
// `nyx/m2-foundation` routes the `m2-lint` word to `run_m2_cli` with no handler behind it. M2-08
// is not written yet. Rather than write a second lint next to Nyx's, this gate FAILS CLOSED:
// it passes only while `pure/m3/` holds no `.omni` module, and rejects any module until the real
// tools are wired in here (flip `M3_GATE_TOOLS_WIRED` in the same commit that calls them).

pub const M3_GATE_TOOLS_WIRED: bool = true;
pub const M3_GATE_MISSING: &str = "none (wired to compile_omni_surface_to_u16 + decode_omni_token_stream + verify_module_for_multi_aot)";

/// Gate verdict for a set of `(path, source)` modules. Ok(n) = n modules passed.
pub fn m3_gate_check(modules: &[(String, String)]) -> Result<usize, String> {
    if modules.is_empty() {
        return Ok(0);
    }
    if !M3_GATE_TOOLS_WIRED {
        let names: Vec<&str> = modules.iter().map(|(p, _)| p.as_str()).collect();
        return Err(format!(
            "error[M3_GATE_UNAVAILABLE] refusing to pass {} pure/m3 module(s) [{}]: {}",
            modules.len(),
            names.join(", "),
            M3_GATE_MISSING
        ));
    }
    for (path, src) in modules {
        let u16s = compile_omni_surface_to_u16(src)
            .map_err(|e| format!("error[M3_GATE_FAIL] {} failed to compile: {}", path, e))?;
        let m = omni_ir::decode_omni_token_stream(&u16s)
            .map_err(|e| format!("error[M3_GATE_FAIL] {} failed to decode: {}", path, e))?;
        omni_verifier::verify_module_for_multi_aot(&m)
            .map_err(|e| format!("error[M3_GATE_FAIL] {} failed multi-AOT/call-graph check: {}", path, e))?;
    }
    Ok(modules.len())
}

/// Every `*.omni` directly under `dir`, sorted by path. A missing directory is an empty set.
pub fn m3_gate_collect(dir: &str) -> Result<Vec<(String, String)>, String> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for ent in rd {
        let p = ent.map_err(|e| format!("m3-gate: {}", e))?.path();
        if p.extension().and_then(|x| x.to_str()) == Some("omni") {
            let s = fs::read_to_string(&p).map_err(|e| format!("m3-gate: cannot read {}: {}", p.display(), e))?;
            out.push((p.display().to_string(), s));
        }
    }
    out.sort();
    Ok(out)
}

/// M3-02 selftest: the gate is vacuous on an empty set, fails closed on any module (including a
/// clean one and a self-recursive one), and, when run from the repo root, gates `pure/m3/` itself.
pub fn run_m3_gate_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    if m3_gate_check(&[]) != Ok(0) {
        return Err(String::from("M3 gate: empty module set must pass vacuously"));
    }
    checks += 1;
    let clean = (String::from("pure/m3/clean.omni"), String::from("fn main(x: i64) -> i64 { return x; }"));
    let rec = (
        String::from("pure/m3/rec.omni"),
        String::from("fn f(x: i64) -> i64 { let y = f(x); return y; }\nfn main(x: i64) -> i64 { let r = f(x); return r; }"),
    );
    if m3_gate_check(&[clean]) != Ok(1) {
        return Err(String::from("M3 gate: clean module must pass"));
    }
    checks += 1;
    match m3_gate_check(&[rec]) {
        Err(e) if e.contains("M3_GATE_FAIL") => checks += 1,
        other => return Err(format!("M3 gate must reject recursive module: {:?}", other)),
    }
    if std::path::Path::new("omni_rust_bootstrap.rs").exists() && std::path::Path::new("spec/m3").is_dir() {
        let mods = m3_gate_collect("pure/m3")?;
        m3_gate_check(&mods)?;
        checks += 1;
    }
    Ok(checks)
}

/// `m3-gate [dir]` (default `pure/m3`): exit 0 only if the gate passes.
pub fn run_m3_gate_cli(argv: &[String]) -> i32 {
    let dir = argv.first().map(|s| s.as_str()).unwrap_or("pure/m3");
    match m3_gate_collect(dir).and_then(|m| m3_gate_check(&m)) {
        Ok(0) => {
            println!("[M3-GATE PASS] 0 modules under {} (vacuous; the gate fails closed on any module: {})", dir, M3_GATE_MISSING);
            0
        }
        Ok(n) => {
            println!("[M3-GATE PASS] {} modules under {}", n, dir);
            0
        }
        Err(e) => {
            eprintln!("[M3-GATE FAIL] {}", e);
            1
        }
    }
}

#[cfg(test)]
mod if_pos_join_slot_tests {
    #[test]
    fn if_pos_join_slot_regression() {
        let n = super::run_if_pos_join_slot_selftests().expect("if_pos join-slot regression");
        assert!(n >= 20, "expected at least 20 if_pos join-slot checks, got {n}");
        eprintln!("if_pos join-slot checks: {n}");
    }

    #[test]
    fn m3_gate_fails_closed() {
        let n = super::run_m3_gate_selftests().expect("M3-02 gate");
        assert!(n >= 3, "expected at least 3 M3 gate checks, got {n}");
    }

    #[test]
    fn m3_fe_lock() {
        let n = super::run_m3_fe_lock_selftests().expect("M3-01 frontend lock");
        assert!(n >= 95, "expected at least 95 M3 lock checks, got {n}");
    }

    #[test]
    fn byte_readback_parity_vera_p1() {
        let n = super::run_byte_readback_parity_selftests().expect("Vera P1 byte readback");
        assert!(n >= 3, "expected at least 3 byte-readback checks, got {n}");
        let (x86, s1) = super::last_byte_readback_native_results();
        eprintln!("byte_readback checks={n} x86={x86:?} stage1={s1:?}");
    }
}
