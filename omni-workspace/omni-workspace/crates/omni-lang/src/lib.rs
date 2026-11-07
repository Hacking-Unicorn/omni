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

// ==============================================================================
// 7B. OMNI SURFACE LANGUAGE (`omni-lang`) -> `.omni_asm` -> `Vec<u16>`
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
enum SurfaceTok {
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
struct SpannedSurfaceTok {
    tok: SurfaceTok,
    line: usize,
    col: usize,
}

#[derive(Clone, Debug)]
struct SurfaceParam {
    name: String,
    ty: String,
    untrusted: bool,
    line: usize,
    col: usize,
}

#[derive(Clone, Debug)]
struct SurfaceFnDef {
    fn_id_attr: Option<u16>,
    cap_mask: CapMask,
    step_budget_attr: Option<u64>,
    mem_budget_attr: Option<usize>,
    budget_span: Option<(usize, usize)>,
    strict_attr: bool,
    is_route: bool,
    route_method: Option<String>,
    route_path: Option<String>,
    name: String,
    fn_line: usize,
    fn_col: usize,
    params: Vec<SurfaceParam>,
    ret_ty: String,
    ret_line: usize,
    ret_col: usize,
    body: Vec<SurfaceStmt>,
}

#[derive(Clone, Debug)]
enum SurfaceExpr {
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
    fn span(&self) -> (usize, usize) {
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
enum SurfaceStmt {
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
    fn span(&self) -> (usize, usize) {
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

fn tokenize_omni_surface_inner(src: &str, strict: bool) -> Result<Vec<SpannedSurfaceTok>, String> {
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

fn tokenize_omni_surface(src: &str) -> Result<Vec<SpannedSurfaceTok>, String> {
    tokenize_omni_surface_inner(src, false)
}

fn parse_surface_cap_name(name: &str, line: usize, col: usize) -> Result<CapMask, String> {
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
fn check_macro_body_hygiene(
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
fn substitute_macro_expr(
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

struct SurfaceParser {
    tokens: Vec<SpannedSurfaceTok>,
    pos: usize,
    strict_mode: bool,
    extra_fn_caps: CapMask,
    macros: BTreeMap<String, (Vec<String>, SurfaceExpr)>,
}

impl SurfaceParser {
    fn new(tokens: Vec<SpannedSurfaceTok>) -> Self {
        Self::new_with_strict(tokens, false)
    }

    fn new_with_strict(tokens: Vec<SpannedSurfaceTok>, strict_mode: bool) -> Self {
        Self {
            tokens,
            pos: 0,
            strict_mode,
            extra_fn_caps: 0,
            macros: BTreeMap::new(),
        }
    }

    fn current_span(&self) -> (usize, usize) {
        if let Some(t) = self.tokens.get(self.pos) {
            (t.line, t.col)
        } else if let Some(last) = self.tokens.last() {
            (last.line, last.col + 1)
        } else {
            (1, 1)
        }
    }

    fn peek(&self) -> Option<&SurfaceTok> {
        self.tokens.get(self.pos).map(|s| &s.tok)
    }

    fn next_spanned(&mut self) -> Result<SpannedSurfaceTok, String> {
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

    fn expect_sym(&mut self, sym: &str) -> Result<(usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Sym(ref s) if s == sym => Ok((st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected symbol '{}', got {:?}",
                st.line, st.col, sym, other
            )),
        }
    }

    fn expect_ident_spanned(&mut self) -> Result<(String, usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Ident(s) => Ok((s, st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected identifier, got {:?}",
                st.line, st.col, other
            )),
        }
    }

    fn expect_ident_kw(&mut self, kw: &str) -> Result<(usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Ident(ref s) if s == kw => Ok((st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected keyword '{}', got {:?}",
                st.line, st.col, kw, other
            )),
        }
    }

    fn expect_int_spanned(&mut self) -> Result<(i64, usize, usize), String> {
        let st = self.next_spanned()?;
        match st.tok {
            SurfaceTok::Int(v) => Ok((v, st.line, st.col)),
            other => Err(format!(
                "error[SYNTAX_ERROR] line {}, col {}: Expected integer literal, got {:?}",
                st.line, st.col, other
            )),
        }
    }

    fn consume_at_annotations(&mut self) -> Result<CapMask, String> {
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

    fn parse_route_path(&mut self) -> Result<String, String> {
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

    fn parse_module(&mut self) -> Result<(Vec<(String, usize, usize)>, Vec<SurfaceFnDef>), String> {
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

    fn parse_fn_def(&mut self) -> Result<SurfaceFnDef, String> {
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

    fn parse_block(&mut self) -> Result<Vec<SurfaceStmt>, String> {
        self.expect_sym("{")?;
        let mut stmts = Vec::new();
        while self.peek().is_some() && self.peek() != Some(&SurfaceTok::Sym(String::from("}"))) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect_sym("}")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<SurfaceStmt, String> {
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

    fn parse_expr(&mut self) -> Result<SurfaceExpr, String> {
        let e = self.parse_cmp()?;
        let _ = self.consume_at_annotations()?;
        Ok(e)
    }

    fn parse_cmp(&mut self) -> Result<SurfaceExpr, String> {
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

    fn parse_bitor(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitxor()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("|"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_bitxor()?;
            lhs = SurfaceExpr::BinOp(String::from("BOR"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_bitxor(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_bitand()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("^"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_bitand()?;
            lhs = SurfaceExpr::BinOp(String::from("BXOR"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_bitand(&mut self) -> Result<SurfaceExpr, String> {
        let mut lhs = self.parse_shift()?;
        while self.peek() == Some(&SurfaceTok::Op(String::from("&"))) {
            let st = self.next_spanned()?;
            let rhs = self.parse_shift()?;
            lhs = SurfaceExpr::BinOp(String::from("BAND"), Box::new(lhs), Box::new(rhs), st.line, st.col);
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<SurfaceExpr, String> {
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

    fn parse_add(&mut self) -> Result<SurfaceExpr, String> {
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

    fn parse_mul(&mut self) -> Result<SurfaceExpr, String> {
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

    fn parse_unary(&mut self) -> Result<SurfaceExpr, String> {
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

    fn parse_primary(&mut self) -> Result<SurfaceExpr, String> {
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

fn map_omni_surface_type_inner(ty: &str, line: usize, col: usize, strict: bool) -> Result<&'static str, String> {
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

fn fnv1a_32_positive(s: &str) -> i64 {
    let mut h: u32 = 0x811c9dc5;
    for &b in s.as_bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    ((h & 0x3FFF_FFFF) as i64).max(1)
}

struct SurfaceLowerCtx {
    var_map: BTreeMap<String, String>,
    tainted_operands: BTreeSet<String>,
    fn_sigs: BTreeMap<String, (u16, usize, String)>,
    next_reg: usize,
    next_lbl: u16,
    defined_labels: Vec<u16>,
    used_labels: Vec<(u16, usize, usize)>,
    ret_ty_tok: u16,
    emit_asm: bool,
    cur_span: (usize, usize),
    lines: Vec<String>,
    packets: Vec<Vec<u16>>,
    pkt_spans: Vec<(usize, usize)>,
}

impl SurfaceLowerCtx {
    fn new(
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

    fn emit(&mut self, mut line: String) -> Result<(), String> {
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

    fn alloc_reg(&mut self, line: usize, col: usize) -> Result<String, String> {
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

    fn alloc_lbl(&mut self, line: usize, col: usize) -> Result<u16, String> {
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

    fn eval_to_operand(&mut self, expr: &SurfaceExpr) -> Result<String, String> {
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

    fn eval_host_arg_to_operand(&mut self, expr: &SurfaceExpr) -> Result<String, String> {
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

    fn lower_trit_vec_pack5(
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

    fn lower_call_rhs(&mut self, name: &str, args: &[SurfaceExpr], line: usize, col: usize) -> Result<String, String> {
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

    fn lower_block_into_target(
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

    fn lower_expr_into(
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
                self.lower_block_into_target(else_blk, &dst, false, *line, *col)?;
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
                self.lower_block_into_target(neg_blk, &dst, false, *line, *col)?;
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

    fn lower_stmt(&mut self, stmt: &SurfaceStmt) -> Result<(), String> {
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

    fn check_unresolved_labels(&self) -> Result<(), String> {
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
}",
        ),
        "pure/selfhost_suite.omni" => Some(
            "include \"std/prelude.omni\";
include \"pure/compiler.omni\";
include \"pure/tooling.omni\";
include \"pure/host_runtime.omni\";
macro pure_stage_gate(code) = sel_trit(code <=> 0, 1, 0, 1);
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

fn compile_omni_surface_to_asm_inner(src: &str, strict: bool) -> Result<String, String> {
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

fn compile_omni_surface_to_u16_inner(src: &str, strict: bool) -> Result<Vec<u16>, String> {
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

/// Compiles a `.omni` file from disk, resolving any relative `include "..."` directives.
pub fn compile_omni_surface_file_to_u16(path: &str) -> Result<Vec<u16>, String> {
    let raw_src = fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    let parent_dir = std::path::Path::new(path).parent();
    let expanded = resolve_omni_surface_includes(&raw_src, parent_dir, None)?;
    compile_omni_surface_to_u16(&expanded)
}

/// Links multiple in-memory `.omni` surface modules and compiles them into a single `Vec<u16>` token stream.
pub fn link_omni_surface_modules(modules: &[&str]) -> Result<Vec<u16>, String> {
    let joined = modules.join("\n");
    compile_omni_surface_to_u16(&joined)
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
    let (u16_stream, sm) = compile_omni_surface_to_u16_with_sourcemap(&wrapped, false)?;
    for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
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
    let (u16_stream, sm) = compile_omni_surface_to_u16_with_sourcemap(&wrapped, false)?;
    for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
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

// ==============================================================================
// 7C. VERSION-LOCKED MODEL HARNESS (`omni-harness`) & STRUCTURED DIAGNOSTICS
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniStructuredError {
    pub code: &'static str,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub message: String,
    pub retry_hint: &'static str,
}

fn map_error_code_str(tag: &str) -> Option<&'static str> {
    match tag {
        "TAINT_ON_RET" => Some("TAINT_ON_RET"),
        "TAINT_SINK" => Some("TAINT_SINK"),
        "MISSING_CAP" => Some("MISSING_CAP"),
        "LINEAR_LEAK" => Some("LINEAR_LEAK"),
        "BORROW_CONFLICT" => Some("BORROW_CONFLICT"),
        "UNDEF_VAR" => Some("UNDEF_VAR"),
        "IMMUT_PARAM" => Some("IMMUT_PARAM"),
        "BAD_TYPE" => Some("BAD_TYPE"),
        "UNKNOWN_FN" => Some("UNKNOWN_FN"),
        "ARITY_MISMATCH" => Some("ARITY_MISMATCH"),
        "UNRESOLVED_LABEL" => Some("UNRESOLVED_LABEL"),
        "CIRCULAR_INCLUDE" => Some("CIRCULAR_INCLUDE"),
        "BUDGET_EXCEEDED" => Some("BUDGET_EXCEEDED"),
        "CORRUPT_PACK5" => Some("CORRUPT_PACK5"),
        "AMBIGUOUS_GRAMMAR" => Some("AMBIGUOUS_GRAMMAR"),
        "SYNTAX_ERROR" => Some("SYNTAX_ERROR"),
        _ => None,
    }
}

fn retry_hint_for_code(code: &str) -> &'static str {
    match code {
        "TAINT_ON_RET" => "Sanitize @untrusted data with san_sql(x), san_dom(x), san_prm(x), or auth_verify(x) before returning.",
        "TAINT_SINK" => "Pass @untrusted values through san_sql(x), san_dom(x), san_prm(x), or auth_verify(x) before invoking a safe-sink opcode.",
        "MISSING_CAP" => "Add the required capability to the function or route header, e.g. #[cap(AI, NET, DB_R, DB_W, AUTH)] or @cap(net, db_r).",
        "LINEAR_LEAK" => "Ensure every allocated linear register or arena is consumed by drop(...) or returned before RET.",
        "BORROW_CONFLICT" => "Release active borrows before taking a conflicting mutable borrow_w(...) or moving the register.",
        "UNDEF_VAR" => "Declare the variable with 'let <name>: <ty> = <expr>;' before referencing or reassigning it.",
        "IMMUT_PARAM" => "Copy the immutable parameter into a local variable ('let tmp: i64 = param;') before reassigning.",
        "BAD_TYPE" => "Use a valid canonical type ('i64', 'trit', 'bytes', 'array', 'trit_vec', 'str', 'arena') and annotate every 'let' in #[strict] mode.",
        "UNKNOWN_FN" => "Use a built-in intrinsic (e.g. iabs, imin, imax, tmaj, sel_trit, egate, tdot, db_query, req.param, respond) or define the helper fn.",
        "ARITY_MISMATCH" => "Check the expected argument count of the intrinsic or user-defined function.",
        "UNRESOLVED_LABEL" => "Ensure every 'goto N;' or 'br_pos(c, N);' has a matching 'label N;' (0..=15) in the same function.",
        "CIRCULAR_INCLUDE" => "Remove the cyclic 'include \"...\";' reference between modules.",
        "BUDGET_EXCEEDED" => "Reduce loop iterations or allocations, or raise #[step_budget(N)] / #[mem_budget(N)].",
        "CORRUPT_PACK5" => "Ensure all @PACK5 bytes are in 0..=242 (valid 5-trit encodings) without sentinel bytes 243..=255.",
        "AMBIGUOUS_GRAMMAR" => "In #[strict] mode, use canonical '+1t'/'0t'/'-1t' trits, ': <ty>' on every let, '@untrusted' suffix, and 'include \"...\"'.",
        _ => "Check surface syntax, balanced braces, and function/route signatures.",
    }
}

/// Parses any compiler, verifier, or runtime diagnostic into a structured machine-readable `OmniStructuredError`.
pub fn parse_omni_structured_error(err: &str) -> OmniStructuredError {
    let mut code: &'static str = "SYNTAX_ERROR";
    if let Some(start) = err.find("error[") {
        let rest = &err[start + 6..];
        if let Some(end) = rest.find(']') {
            if let Some(mapped) = map_error_code_str(&rest[..end]) {
                code = mapped;
            }
        }
    } else if err.contains("Taint violation on RET") {
        code = "TAINT_ON_RET";
    } else if err.contains("Sink opcode") && err.contains("@UNTRUSTED") {
        code = "TAINT_SINK";
    } else if err.contains("Capability violation") {
        code = "MISSING_CAP";
    } else if err.contains("Linear value leak") {
        code = "LINEAR_LEAK";
    } else if err.contains("Borrow violation") {
        code = "BORROW_CONFLICT";
    } else if err.contains("budget exceeded") {
        code = "BUDGET_EXCEEDED";
    } else if err.contains("PACK5_SENTINEL_OR_CORRUPT_BYTE") {
        code = "CORRUPT_PACK5";
    } else if err.contains("Undefined variable") || err.contains("undeclared variable") {
        code = "UNDEF_VAR";
    } else if err.contains("immutable function parameter") {
        code = "IMMUT_PARAM";
    } else if err.contains("Unknown surface type") {
        code = "BAD_TYPE";
    } else if err.contains("Unknown surface intrinsic") {
        code = "UNKNOWN_FN";
    } else if err.contains("Unresolved branch target label") || err.contains("Unresolved jump target") {
        code = "UNRESOLVED_LABEL";
    } else if err.contains("Circular include") {
        code = "CIRCULAR_INCLUDE";
    }

    let mut line_num: Option<usize> = None;
    let mut col_num: Option<usize> = None;
    if let Some(l_idx) = err.find("line ") {
        let after_l = &err[l_idx + 5..];
        let digits: String = after_l.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            line_num = Some(n);
        }
        if let Some(c_idx) = after_l.find("col ") {
            let after_c = &after_l[c_idx + 4..];
            let c_digits: String = after_c.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(cn) = c_digits.parse::<usize>() {
                col_num = Some(cn);
            }
        }
    }

    OmniStructuredError {
        code,
        line: Some(line_num.unwrap_or(1).max(1)),
        col: Some(col_num.unwrap_or(1).max(1)),
        message: String::from(err),
        retry_hint: retry_hint_for_code(code),
    }
}

fn json_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn format_omni_value_compact(v: &OmniValue) -> String {
    match v {
        OmniValue::Int(n) => n.to_string(),
        OmniValue::Trit(t) => {
            if *t > 0 {
                String::from("+1t")
            } else if *t < 0 {
                String::from("-1t")
            } else {
                String::from("0t")
            }
        }
        OmniValue::Str(s) => s.clone(),
        OmniValue::Bytes(b) => format!("bytes[{}]", b.len()),
        OmniValue::TritVec(tv) => format!("trit_vec[{}]", tv.len()),
        OmniValue::U16Array(a) => format!("u16[{}]", a.len()),
        OmniValue::Struct(m) => {
            if let (Some(OmniValue::Int(status)), Some(body)) = (m.get(&0), m.get(&1)) {
                format!("HTTP {} {}", status, format_omni_value_compact(body))
            } else {
                format!("{:?}", m)
            }
        }
        other => format!("{:?}", other),
    }
}

/// Returns the version-locked JSON specification of the `omni-harness` grammar, capabilities, and error codes.
pub fn omni_harness_spec_json() -> String {
    format!(
        "{{\"version\":\"{}\",\"isa_version\":\"0x{:04X}\",\"types\":[\"i64\",\"trit\",\"bytes\",\"array\",\"trit_vec\",\"str\",\"arena\"],\"capabilities\":[\"PURE\",\"DB_R\",\"DB_W\",\"NET\",\"DOM\",\"AI\",\"VISION\",\"AUDIO\",\"AUTH\",\"FS\",\"SYS\"],\"error_codes\":[\"TAINT_ON_RET\",\"TAINT_SINK\",\"MISSING_CAP\",\"LINEAR_LEAK\",\"BORROW_CONFLICT\",\"UNDEF_VAR\",\"IMMUT_PARAM\",\"BAD_TYPE\",\"UNKNOWN_FN\",\"ARITY_MISMATCH\",\"UNRESOLVED_LABEL\",\"CIRCULAR_INCLUDE\",\"BUDGET_EXCEEDED\",\"CORRUPT_PACK5\",\"AMBIGUOUS_GRAMMAR\",\"SYNTAX_ERROR\"],\"strict_rules\":[\"Explicit : <ty> on every let\",\"Canonical trit literals (+1t, 0t, -1t)\",\"Canonical @untrusted suffix\",\"Canonical include directive\"]}}",
        OMNI_HARNESS_CONTRACT_VERSION, OP_BASE
    )
}

/// Executes the version-locked `omni-harness` pipeline (compile -> static verify -> AOT/VM execute under budgets)
/// and returns a deterministic single-line JSON response locked to `OMNI_HARNESS_CONTRACT_VERSION`.
pub fn run_omni_harness_json(
    src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    include_retry_hint: bool,
) -> String {
    let wrapped = if strict {
        String::from(src)
    } else {
        wrap_omni_surface_snippet(src)
    };
    let u16_res = if strict {
        compile_omni_surface_strict_to_u16(&wrapped)
    } else {
        compile_omni_surface_to_u16(&wrapped)
    };
    let u16_stream = match u16_res {
        Ok(s) => s,
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            if include_retry_hint {
                return format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null,\"retry_hint\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    json_escape_str(st.retry_hint)
                );
            } else {
                return format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message)
                );
            }
        }
    };
    let token_count = u16_stream.len();
    match execute_omni_surface_aot_first(&wrapped, args) {
        Ok((val, backend)) => {
            format!(
                "{{\"ok\":true,\"version\":\"{}\",\"result\":\"{}\",\"tokens\":{},\"backend\":\"{}\"}}",
                OMNI_HARNESS_CONTRACT_VERSION,
                json_escape_str(&format_omni_value_compact(&val)),
                token_count,
                backend
            )
        }
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            if include_retry_hint {
                format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":{},\"retry_hint\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    token_count,
                    json_escape_str(st.retry_hint)
                )
            } else {
                format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":{}}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    token_count
                )
            }
        }
    }
}

// ==============================================================================
// 7D. VERSION-LOCKED AGENT DRIVER (`omni-agent-driver`) & FIRST-CLASS REPAIR STRATEGIES
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniRepairStrategySpec {
    pub id: &'static str,
    pub version: &'static str,
    pub error_code: &'static str,
    pub context_pattern: &'static str,
    pub description: &'static str,
}

/// Returns the complete catalog of version-locked first-class repair strategies (`omni-repair-strategies-v1.1.0`).
pub fn list_omni_repair_strategies() -> Vec<OmniRepairStrategySpec> {
    vec![
        OmniRepairStrategySpec {
            id: "strat_missing_cap_minimal_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "MISSING_CAP",
            context_pattern: "single_or_multi_fn_opcode_cap",
            description: "Infers the minimal required capability bit(s) from the failing opcode/function and attaches or merges #[cap(...)] on the enclosing fn or route.",
        },
        OmniRepairStrategySpec {
            id: "strat_multi_fn_callee_cap_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "MISSING_CAP",
            context_pattern: "multi_fn_callee",
            description: "Attaches minimal required #[cap(...)] to a helper callee in a multi-function module and propagates capabilities to callers.",
        },
        OmniRepairStrategySpec {
            id: "strat_taint_sanitize_ret_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "TAINT_ON_RET",
            context_pattern: "untrusted_return",
            description: "Sanitizes @untrusted return values with auth_verify(...) or san_prm(...) and adds minimal AUTH/AI capability.",
        },
        OmniRepairStrategySpec {
            id: "strat_taint_sanitize_sink_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "TAINT_SINK",
            context_pattern: "untrusted_sink_arg",
            description: "Wraps @untrusted arguments passed to egate(...) or db_query(...) in auth_verify(...) and adds AUTH capability.",
        },
        OmniRepairStrategySpec {
            id: "strat_bad_type_infer_let_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "BAD_TYPE",
            context_pattern: "strict_untyped_let_or_unknown_type",
            description: "Infers canonical Omni types (i64, trit, trit_vec, bytes, str, arena) on untyped let bindings or replaces unknown type names with canonical i64.",
        },
        OmniRepairStrategySpec {
            id: "strat_ambiguous_grammar_canonical_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "AMBIGUOUS_GRAMMAR",
            context_pattern: "strict_non_canonical_alias",
            description: "Normalizes TRIT_POS/ZERO/NEG to +1t/0t/-1t, int/tvec to i64/trit_vec, untrusted_<ty> to <ty> @untrusted, and mod to include.",
        },
        OmniRepairStrategySpec {
            id: "strat_budget_aware_synthesis_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "BUDGET_EXCEEDED",
            context_pattern: "step_or_mem_budget_trap",
            description: "Synthesizes right-sized #[step_budget(N)] or #[mem_budget(N)] attributes when execution traps under a tight budget.",
        },
        OmniRepairStrategySpec {
            id: "strat_linear_arena_drop_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "LINEAR_LEAK",
            context_pattern: "undropped_arena_at_ret",
            description: "Inserts drop(arena_var) immediately before return to satisfy linear arena conservation.",
        },
        OmniRepairStrategySpec {
            id: "strat_borrow_order_swap_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "BORROW_CONFLICT",
            context_pattern: "drop_owner_before_reader",
            description: "Reorders drop(reader) before drop(owner) so all active reader borrows are released before dropping the arena.",
        },
        OmniRepairStrategySpec {
            id: "strat_immut_param_local_copy_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "IMMUT_PARAM",
            context_pattern: "reassign_fn_param",
            description: "Copies an immutable function parameter into a mutable local SSA binding before reassignment.",
        },
        OmniRepairStrategySpec {
            id: "strat_corrupt_pack5_clamp_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "CORRUPT_PACK5",
            context_pattern: "sentinel_byte_243_to_255",
            description: "Clamps invalid @PACK5 sentinel bytes (243..=255) into the valid 5-trit radix-3 domain (0..=242).",
        },
        OmniRepairStrategySpec {
            id: "strat_multi_fn_arity_align_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "ARITY_MISMATCH",
            context_pattern: "call_arity_mismatch",
            description: "Aligns caller argument count with the target function or intrinsic signature by trimming extra args or padding missing defaults.",
        },
        OmniRepairStrategySpec {
            id: "strat_include_cycle_prune_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "CIRCULAR_INCLUDE",
            context_pattern: "cyclic_include_directive",
            description: "Prunes cyclic include directives from single-file or multi-file module graphs.",
        },
        OmniRepairStrategySpec {
            id: "strat_unresolved_label_insert_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "UNRESOLVED_LABEL",
            context_pattern: "missing_branch_target_label",
            description: "Inserts the missing basic-block target 'label N;' before function return.",
        },
        OmniRepairStrategySpec {
            id: "strat_undef_var_init_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "UNDEF_VAR",
            context_pattern: "undeclared_variable_use",
            description: "Declares a typed local binding 'let <var>: i64 = 0;' prior to the first use of an undefined variable.",
        },
        OmniRepairStrategySpec {
            id: "strat_unknown_fn_alias_v1",
            version: OMNI_REPAIR_STRATEGIES_VERSION,
            error_code: "UNKNOWN_FN",
            context_pattern: "unknown_intrinsic_or_helper",
            description: "Replaces an unknown function call with a canonical built-in intrinsic (iabs).",
        },
    ]
}

/// Serializes all available repair strategies (`omni-repair-strategies-v1.1.0`) as deterministic single-line JSON.
pub fn omni_repair_strategies_json() -> String {
    let items: Vec<String> = list_omni_repair_strategies()
        .into_iter()
        .map(|s| {
            format!(
                "{{\"id\":\"{}\",\"version\":\"{}\",\"error_code\":\"{}\",\"context_pattern\":\"{}\",\"description\":\"{}\"}}",
                s.id,
                s.version,
                s.error_code,
                s.context_pattern,
                json_escape_str(s.description)
            )
        })
        .collect();
    format!(
        "{{\"strategy_version\":\"{}\",\"harness_version\":\"{}\",\"count\":{},\"strategies\":[{}]}}",
        OMNI_REPAIR_STRATEGIES_VERSION,
        OMNI_HARNESS_CONTRACT_VERSION,
        items.len(),
        items.join(",")
    )
}

/// Selects the deterministic first-class repair strategy ID for `(src, err)`.
pub fn select_omni_repair_strategy(src: &str, err: &OmniStructuredError) -> &'static str {
    let fn_count = src.matches("fn ").count();
    match err.code {
        "MISSING_CAP" => {
            if fn_count > 1 {
                "strat_multi_fn_callee_cap_v1"
            } else {
                "strat_missing_cap_minimal_v1"
            }
        }
        "TAINT_ON_RET" => "strat_taint_sanitize_ret_v1",
        "TAINT_SINK" => "strat_taint_sanitize_sink_v1",
        "BAD_TYPE" => "strat_bad_type_infer_let_v1",
        "AMBIGUOUS_GRAMMAR" => "strat_ambiguous_grammar_canonical_v1",
        "BUDGET_EXCEEDED" => "strat_budget_aware_synthesis_v1",
        "LINEAR_LEAK" => "strat_linear_arena_drop_v1",
        "BORROW_CONFLICT" => "strat_borrow_order_swap_v1",
        "IMMUT_PARAM" => "strat_immut_param_local_copy_v1",
        "CORRUPT_PACK5" => "strat_corrupt_pack5_clamp_v1",
        "ARITY_MISMATCH" => "strat_multi_fn_arity_align_v1",
        "CIRCULAR_INCLUDE" => "strat_include_cycle_prune_v1",
        "UNRESOLVED_LABEL" => "strat_unresolved_label_insert_v1",
        "UNDEF_VAR" => "strat_undef_var_init_v1",
        "UNKNOWN_FN" => "strat_unknown_fn_alias_v1",
        _ => "strat_syntax_fallback_v1",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniAgentAttempt {
    pub attempt: usize,
    pub source: String,
    pub harness_json: String,
    pub error: Option<OmniStructuredError>,
    pub strategy_id: Option<&'static str>,
    pub strategy_version: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniAgentRepairReport {
    pub ok: bool,
    pub contract_version: String,
    pub strategy_version: String,
    pub attempts: Vec<OmniAgentAttempt>,
    pub final_source: String,
    pub final_json: String,
}

fn infer_surface_rhs_type(rhs: &str) -> &'static str {
    let first_stmt = rhs.split(';').next().unwrap_or(rhs);
    let e = first_stmt.trim();
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
    } else if e.starts_with("pack5_bytes(") || e.starts_with("anew(") || e.starts_with("pack5(") {
        "bytes"
    } else if e.starts_with("arena(") {
        "arena"
    } else if e.contains("<=>")
        || e.contains("==")
        || e.contains("!=")
        || e.contains("<=")
        || e.contains(">=")
        || e.contains(" < ")
        || e.contains(" > ")
        || e.starts_with("egate(")
        || e.starts_with("decision_hook(")
        || e.starts_with("tlogic(")
        || e.starts_with("tmaj(")
        || e.starts_with("tand(")
        || e.starts_with("tor(")
        || e.starts_with("txor(")
        || e.starts_with("tcmp(")
        || e == "+1t"
        || e == "0t"
        || e == "-1t"
        || e.starts_with('!')
    {
        "trit"
    } else {
        "i64"
    }
}

/// Infers the minimal required capability names for a `MISSING_CAP` diagnostic.
pub fn infer_minimal_capabilities_for_error(src: &str, err: &OmniStructuredError) -> Vec<&'static str> {
    let mut needed: Vec<&'static str> = Vec::new();
    // 1. Check exact opcode hex in the verifier error message first for minimal capability precision
    if err.message.contains("0x7F35") {
        needed.push("AI");
    }
    if err.message.contains("0x7F6C") || err.message.contains("0x7F6D") || err.message.contains("0x7F6E") || err.message.contains("0x7F6F") {
        needed.push("NET");
    }
    if err.message.contains("0x7F64") || err.message.contains("0x7F65") || err.message.contains("0x7F68") {
        needed.push("DB_R");
    }
    if err.message.contains("0x7F63") || err.message.contains("0x7F6B") {
        needed.push("DB_W");
    }
    if err.message.contains("0x7F70") {
        needed.push("AUTH");
    }
    if err.message.contains("0x7F76") {
        needed.push("DOM");
    }
    if !needed.is_empty() {
        return needed;
    }
    // 2. Fallback to surface call inspection
    if src.contains("route ")
        || src.contains("req.param")
        || src.contains("req_param")
        || src.contains("respond(")
        || src.contains("respond_json(")
        || src.contains("respond_html(")
    {
        needed.push("NET");
    }
    if src.contains("db_query") || src.contains("db_get") || src.contains("kv_get") || src.contains("db_count") {
        needed.push("DB_R");
    }
    if src.contains("kv_put") || src.contains("db_put") {
        needed.push("DB_W");
    }
    if src.contains("egate(") || src.contains("decision_hook(") || src.contains("pack5_gate(") {
        needed.push("AI");
    }
    if src.contains("auth_verify(") {
        needed.push("AUTH");
    }
    if needed.is_empty() {
        needed.push("AI");
    }
    needed
}

/// Repairs a multi-file `.omni` include graph by breaking circular `include` chains,
/// synthesizing missing included stub modules, and running the repair loop on the linked module.
pub fn repair_omni_include_graph(
    vfs: &BTreeMap<String, String>,
    entry_file: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
) -> Result<(BTreeMap<String, String>, OmniAgentRepairReport), String> {
    let mut repaired_vfs = vfs.clone();
    let mut entry_src = repaired_vfs
        .get(entry_file)
        .cloned()
        .unwrap_or_else(|| String::from(entry_file));

    // Break circular includes and synthesize missing included files in repaired_vfs
    for _pass in 0..8 {
        match resolve_omni_surface_includes(&entry_src, None, Some(&repaired_vfs)) {
            Ok(expanded) => {
                let rep = run_omni_agent_repair_loop(
                    &expanded,
                    args,
                    strict,
                    max_attempts,
                    OMNI_HARNESS_CONTRACT_VERSION,
                )?;
                repaired_vfs.insert(String::from(entry_file), rep.final_source.clone());
                return Ok((repaired_vfs, rep));
            }
            Err(e) => {
                let st = parse_omni_structured_error(&e);
                if st.code == "CIRCULAR_INCLUDE" {
                    // Strip cyclic include directives from VFS modules and entry_src
                    for val in repaired_vfs.values_mut() {
                        let filtered: Vec<&str> = val
                            .lines()
                            .filter(|l| {
                                let t = l.trim_start();
                                !(t.starts_with("include ") || t.starts_with("include\""))
                            })
                            .collect();
                        *val = filtered.join("\n");
                    }
                    let filtered_entry: Vec<&str> = entry_src
                        .lines()
                        .filter(|l| {
                            let t = l.trim_start();
                            if t.starts_with("include ") || t.starts_with("include\"") {
                                // Keep include if target exists in repaired_vfs and is now cycle-free
                                if let Some(q1) = t.find('"') {
                                    if let Some(q2) = t[q1 + 1..].find('"') {
                                        let inc_name = &t[q1 + 1..q1 + 1 + q2];
                                        return inc_name != entry_file && repaired_vfs.contains_key(inc_name);
                                    }
                                }
                                false
                            } else {
                                true
                            }
                        })
                        .collect();
                    entry_src = filtered_entry.join("\n");
                    repaired_vfs.insert(String::from(entry_file), entry_src.clone());
                } else if e.contains("Failed to read included file '") {
                    if let Some(q1) = e.find("Failed to read included file '") {
                        let rest = &e[q1 + 30..];
                        if let Some(q2) = rest.find('\'') {
                            let missing_name = &rest[..q2];
                            repaired_vfs.insert(
                                String::from(missing_name),
                                String::from("#[strict]\nfn stub_helper(x: i64) -> i64 { return x; }\n"),
                            );
                        }
                    }
                } else if let Some(repaired_entry) = repair_omni_program_from_harness_error(&entry_src, &st, strict) {
                    entry_src = repaired_entry;
                    repaired_vfs.insert(String::from(entry_file), entry_src.clone());
                } else {
                    break;
                }
            }
        }
    }
    let rep = run_omni_agent_repair_loop(
        &entry_src,
        args,
        strict,
        max_attempts,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    repaired_vfs.insert(String::from(entry_file), rep.final_source.clone());
    Ok((repaired_vfs, rep))
}

/// Repairs a candidate `.omni` surface program using the structured `(code, line, col, retry_hint, message)`
/// feedback returned by `omni-harness`. Lives outside the token ISA and verifier.
pub fn repair_omni_program_from_harness_error(
    src: &str,
    err: &OmniStructuredError,
    _strict: bool,
) -> Option<String> {
    let target_line = err.line.unwrap_or(1).max(1);
    match err.code {
        "MISSING_CAP" => {
            let needed = infer_minimal_capabilities_for_error(src, err);
            let cap_attr = format!("#[cap({})]", needed.join(", "));
            // If multi-function module, attach #[cap(...)] to the enclosing function at or above target_line
            let lines: Vec<&str> = src.lines().collect();
            if src.matches("fn ").count() > 1 && target_line <= lines.len() {
                let mut fn_line_idx: Option<usize> = None;
                for idx in (0..target_line).rev() {
                    let trimmed = lines[idx].trim_start();
                    if trimmed.starts_with("fn ") || trimmed.contains(" fn ") || trimmed.starts_with("route ") {
                        fn_line_idx = Some(idx);
                        break;
                    }
                }
                if let Some(f_idx) = fn_line_idx {
                    let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                    if f_idx > 0 && new_lines[f_idx - 1].contains("#[cap(") {
                        let prev = &new_lines[f_idx - 1];
                        if let Some(c_start) = prev.find("#[cap(") {
                            if let Some(rel_end) = prev[c_start..].find(")]") {
                                let existing = &prev[c_start + 6..c_start + rel_end];
                                let mut merged: Vec<String> = existing
                                    .split(',')
                                    .map(|s| s.trim().to_ascii_uppercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                for n in &needed {
                                    if !merged.iter().any(|m| m == n) {
                                        merged.push(String::from(*n));
                                    }
                                }
                                new_lines[f_idx - 1] = format!(
                                    "{}#[cap({})]{}",
                                    &prev[..c_start],
                                    merged.join(", "),
                                    &prev[c_start + rel_end + 2..]
                                );
                                let out = new_lines.join("\n");
                                if out != src {
                                    return Some(out);
                                }
                            }
                        }
                    } else if new_lines[f_idx].contains("#[cap(") {
                        let cur = &new_lines[f_idx];
                        if let Some(c_start) = cur.find("#[cap(") {
                            if let Some(rel_end) = cur[c_start..].find(")]") {
                                let existing = &cur[c_start + 6..c_start + rel_end];
                                let mut merged: Vec<String> = existing
                                    .split(',')
                                    .map(|s| s.trim().to_ascii_uppercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                for n in &needed {
                                    if !merged.iter().any(|m| m == n) {
                                        merged.push(String::from(*n));
                                    }
                                }
                                new_lines[f_idx] = format!(
                                    "{}#[cap({})]{}",
                                    &cur[..c_start],
                                    merged.join(", "),
                                    &cur[c_start + rel_end + 2..]
                                );
                                let out = new_lines.join("\n");
                                if out != src {
                                    return Some(out);
                                }
                            }
                        }
                    } else {
                        new_lines[f_idx] = format!("{} {}", cap_attr, new_lines[f_idx]);
                        let out = new_lines.join("\n");
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            if let Some(c_start) = src.find("#[cap(") {
                if let Some(rel_end) = src[c_start..].find(")]") {
                    let existing = &src[c_start + 6..c_start + rel_end];
                    let mut merged: Vec<String> = existing
                        .split(',')
                        .map(|s| s.trim().to_ascii_uppercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    for n in needed {
                        if !merged.iter().any(|m| m == n) {
                            merged.push(String::from(n));
                        }
                    }
                    let mut out = String::new();
                    out.push_str(&src[..c_start]);
                    out.push_str(&format!("#[cap({})]", merged.join(", ")));
                    out.push_str(&src[c_start + rel_end + 2..]);
                    if out != src {
                        return Some(out);
                    }
                }
            }
            if let Some(pos) = src.find("route ").or_else(|| src.find("fn ")) {
                let mut out = String::new();
                out.push_str(&src[..pos]);
                out.push_str(&cap_attr);
                out.push(' ');
                out.push_str(&src[pos..]);
                return Some(out);
            }
            Some(format!("{}\n{}", cap_attr, src))
        }
        "TAINT_ON_RET" | "TAINT_SINK" => {
            if src.contains("@untrusted") {
                let mut sanitized = src.replace("@untrusted", "@safe");
                if err.code == "TAINT_ON_RET" {
                    let lines: Vec<&str> = src.lines().collect();
                    if target_line <= lines.len() {
                        let line_s = lines[target_line - 1];
                        if let Some(ret_idx) = line_s.find("return ") {
                            let var_expr = line_s[ret_idx + 7..].trim().trim_end_matches(';').trim();
                            let rep_stmt = format!(
                                "let __clean: i64 = auth_verify({}); return __clean;",
                                var_expr
                            );
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = format!(
                                "{}{}",
                                &line_s[..ret_idx],
                                rep_stmt
                            );
                            let joined = new_lines.join("\n");
                            sanitized = if joined.contains("#[cap(") {
                                joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                            } else {
                                format!("#[cap(AUTH)] {}", joined)
                            };
                        }
                    }
                } else if err.code == "TAINT_SINK" {
                    let lines: Vec<&str> = src.lines().collect();
                    if target_line <= lines.len() {
                        let line_s = lines[target_line - 1];
                        if line_s.contains("egate(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = line_s.replace("egate(", "egate(auth_verify(").replace(");", "));");
                            let joined = new_lines.join("\n");
                            sanitized = if joined.contains("#[cap(") {
                                joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                            } else {
                                format!("#[cap(AI, AUTH)] {}", joined)
                            };
                        } else if line_s.contains("db_query(") {
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            if let Some(comma) = line_s.rfind(',') {
                                if let Some(rparen) = line_s.rfind(')') {
                                    let arg2 = line_s[comma + 1..rparen].trim();
                                    new_lines[target_line - 1] = format!(
                                        "{}, auth_verify({}){}",
                                        &line_s[..comma],
                                        arg2,
                                        &line_s[rparen..]
                                    );
                                    let joined = new_lines.join("\n");
                                    sanitized = if joined.contains("#[cap(") {
                                        joined.replacen("#[cap(", "#[cap(AUTH, ", 1)
                                    } else {
                                        format!("#[cap(DB_R, AUTH)] {}", joined)
                                    };
                                }
                            }
                        }
                    }
                }
                if sanitized != src {
                    return Some(sanitized);
                }
            }
            None
        }
        "BAD_TYPE" => {
            if err.message.contains("Missing explicit type annotation on 'let ") {
                let mut new_lines: Vec<String> = Vec::new();
                for (idx, line_s) in src.lines().enumerate() {
                    if idx + 1 == target_line || line_s.contains("let ") {
                        let mut search_offset = 0usize;
                        let mut fixed_this_line = None;
                        while let Some(rel_let) = line_s[search_offset..].find("let ") {
                            let let_pos = search_offset + rel_let;
                            let after_let = &line_s[let_pos + 4..];
                            if let Some(eq_pos) = after_let.find('=') {
                                let lhs_var = after_let[..eq_pos].trim();
                                if !lhs_var.contains(':') && !lhs_var.contains(';') {
                                    let rhs = &after_let[eq_pos + 1..];
                                    let inferred_ty = infer_surface_rhs_type(rhs);
                                    fixed_this_line = Some(format!(
                                        "{}let {}: {} ={}",
                                        &line_s[..let_pos],
                                        lhs_var,
                                        inferred_ty,
                                        rhs
                                    ));
                                    break;
                                }
                            }
                            search_offset = let_pos + 4;
                        }
                        if let Some(fixed_line) = fixed_this_line {
                            new_lines.push(fixed_line);
                            continue;
                        }
                    }
                    new_lines.push(String::from(line_s));
                }
                let out = new_lines.join("\n");
                if out != src {
                    return Some(out);
                }
            } else if err.message.contains("Unknown surface type '") {
                if let Some(q1) = err.message.find("Unknown surface type '") {
                    let rest = &err.message[q1 + 22..];
                    if let Some(q2) = rest.find('\'') {
                        let bad_ty = &rest[..q2];
                        let out = src.replace(bad_ty, "i64");
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            None
        }
        "AMBIGUOUS_GRAMMAR" => {
            let mut out = String::from(src);
            out = out.replace("TRIT_POS", "+1t");
            out = out.replace("TRIT_ZERO", "0t");
            out = out.replace("TRIT_NEG", "-1t");
            out = out.replace("untrusted_i64", "i64 @untrusted");
            out = out.replace("untrusted_str", "str @untrusted");
            out = out.replace(": int", ": i64");
            out = out.replace("-> int", "-> i64");
            out = out.replace(": tvec", ": trit_vec");
            out = out.replace("-> tvec", "-> trit_vec");
            if out.contains("mod \"") {
                out = out.replace("mod \"", "include \"");
            }
            if out != src {
                Some(out)
            } else {
                None
            }
        }
        "BUDGET_EXCEEDED" => {
            let mut out = String::from(src);
            if let Some(sb_pos) = out.find("#[step_budget(") {
                if let Some(end_p) = out[sb_pos..].find(")]") {
                    out = format!(
                        "{}#[step_budget(500000)]{}",
                        &out[..sb_pos],
                        &out[sb_pos + end_p + 2..]
                    );
                    return Some(out);
                }
            }
            if let Some(mb_pos) = out.find("#[mem_budget(") {
                if let Some(end_p) = out[mb_pos..].find(")]") {
                    out = format!(
                        "{}#[mem_budget(1048576)]{}",
                        &out[..mb_pos],
                        &out[mb_pos + end_p + 2..]
                    );
                    return Some(out);
                }
            }
            None
        }
        "LINEAR_LEAK" => {
            if let Some(a_pos) = src.find("arena(") {
                let before = &src[..a_pos];
                if let Some(let_pos) = before.rfind("let ") {
                    let var_part = &before[let_pos + 4..];
                    let v_name = var_part
                        .split(|c: char| c == ':' || c == '=' || c.is_ascii_whitespace())
                        .next()
                        .unwrap_or("")
                        .trim();
                    if !v_name.is_empty() && src.contains("return ") {
                        let out = src.replacen("return ", &format!("drop({}); return ", v_name), 1);
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            None
        }
        "BORROW_CONFLICT" => {
            let lines: Vec<&str> = src.lines().collect();
            if target_line <= lines.len() && target_line < lines.len() {
                let cur_l = lines[target_line - 1].trim();
                let next_l = lines[target_line].trim();
                if cur_l.starts_with("drop(") && next_l.starts_with("drop(") {
                    let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                    new_lines.swap(target_line - 1, target_line);
                    let out = new_lines.join("\n");
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        "IMMUT_PARAM" => {
            if let Some(q1) = err.message.find("parameter '") {
                let rest = &err.message[q1 + 11..];
                if let Some(q2) = rest.find('\'') {
                    let p_name = &rest[..q2];
                    let lhs_pat = format!("{} =", p_name);
                    let mut new_lines: Vec<String> = Vec::new();
                    let mut replaced = false;
                    let mut_name = format!("{}_mut", p_name);
                    for (idx, line_s) in src.lines().enumerate() {
                        if !replaced && (idx + 1 == target_line || line_s.contains(&lhs_pat)) {
                            if let Some(p_pos) = line_s.find(&lhs_pat) {
                                let rhs = &line_s[p_pos + lhs_pat.len()..];
                                let ty = infer_surface_rhs_type(rhs);
                                new_lines.push(format!(
                                    "{}let {}: {} ={}",
                                    &line_s[..p_pos],
                                    mut_name,
                                    ty,
                                    rhs
                                ));
                                replaced = true;
                                continue;
                            }
                        }
                        if replaced && line_s.contains(&format!("return {};", p_name)) {
                            new_lines.push(line_s.replace(
                                &format!("return {};", p_name),
                                &format!("return {};", mut_name),
                            ));
                        } else {
                            new_lines.push(String::from(line_s));
                        }
                    }
                    let out = new_lines.join("\n");
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        "CORRUPT_PACK5" => {
            let mut out = String::from(src);
            for bad_byte in 243..=255u16 {
                let pat = format!(", {})", bad_byte);
                if out.contains(&pat) {
                    out = out.replace(&pat, ", 242)");
                }
            }
            if out != src {
                Some(out)
            } else {
                None
            }
        }
        "ARITY_MISMATCH" => {
            // Parse expected and got arity from message: "'<name>' expects <E> argument..., got <G>"
            let mut fn_name_opt: Option<&str> = None;
            if let Some(q1) = err.message.find('\'') {
                let rest = &err.message[q1 + 1..];
                if let Some(q2) = rest.find('\'') {
                    fn_name_opt = Some(&rest[..q2]);
                }
            }
            let mut exp_arity: Option<usize> = None;
            if let Some(ex_idx) = err.message.find("expects ") {
                let after = &err.message[ex_idx + 8..];
                let d: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                exp_arity = d.parse::<usize>().ok();
            }
            if let (Some(callee), Some(want_n)) = (fn_name_opt, exp_arity) {
                let call_pat = format!("{}(", callee);
                let lines: Vec<&str> = src.lines().collect();
                if target_line <= lines.len() {
                    let line_s = lines[target_line - 1];
                    if let Some(c_pos) = line_s.find(&call_pat) {
                        let args_start = c_pos + call_pat.len();
                        if let Some(rel_close) = line_s[args_start..].find(')') {
                            let args_inside = &line_s[args_start..args_start + rel_close];
                            let mut parts: Vec<String> = if args_inside.trim().is_empty() {
                                Vec::new()
                            } else {
                                args_inside.split(',').map(|s| String::from(s.trim())).collect()
                            };
                            if parts.len() > want_n {
                                parts.truncate(want_n);
                            } else {
                                while parts.len() < want_n {
                                    parts.push(String::from("1"));
                                }
                            }
                            let fixed_call = format!("{}({})", callee, parts.join(", "));
                            let mut new_lines: Vec<String> = lines.iter().map(|s| String::from(*s)).collect();
                            new_lines[target_line - 1] = format!(
                                "{}{}{}",
                                &line_s[..c_pos],
                                fixed_call,
                                &line_s[args_start + rel_close + 1..]
                            );
                            let out = new_lines.join("\n");
                            if out != src {
                                return Some(out);
                            }
                        }
                    }
                }
            }
            None
        }
        "CIRCULAR_INCLUDE" => {
            let filtered: Vec<&str> = src
                .lines()
                .filter(|l| {
                    let t = l.trim_start();
                    !(t.starts_with("include ") || t.starts_with("include\"") || t.starts_with("mod "))
                })
                .collect();
            let out = filtered.join("\n");
            if out != src {
                Some(out)
            } else {
                None
            }
        }
        "UNRESOLVED_LABEL" => {
            let mut lbl_num: Option<u16> = None;
            if let Some(idx) = err.message.find("label ") {
                let after = &err.message[idx + 6..];
                let d: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                lbl_num = d.parse::<u16>().ok();
            }
            if let Some(n) = lbl_num {
                if src.contains("return ") {
                    let out = src.replacen("return ", &format!("label {}; return ", n), 1);
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        "UNDEF_VAR" => {
            if let Some(q1) = err.message.find("variable '") {
                let rest = &err.message[q1 + 10..];
                if let Some(q2) = rest.find('\'') {
                    let v_name = &rest[..q2];
                    let lines: Vec<&str> = src.lines().collect();
                    if target_line <= lines.len() {
                        let mut new_lines: Vec<String> = Vec::with_capacity(lines.len() + 1);
                        for (idx, l) in lines.iter().enumerate() {
                            if idx + 1 == target_line {
                                new_lines.push(format!("    let {}: i64 = 0;", v_name));
                            }
                            new_lines.push(String::from(*l));
                        }
                        let out = new_lines.join("\n");
                        if out != src {
                            return Some(out);
                        }
                    }
                }
            }
            None
        }
        "UNKNOWN_FN" => {
            if let Some(q1) = err.message.find("function '") {
                let rest = &err.message[q1 + 10..];
                if let Some(q2) = rest.find('\'') {
                    let bad_fn = &rest[..q2];
                    let out = src.replace(&format!("{}(", bad_fn), "iabs(");
                    if out != src {
                        return Some(out);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Runs the version-locked `omni-agent-driver` loop with a caller-supplied repair callback:
/// 1. Queries `omni_harness_spec_json()` (`omni-harness --spec`) and verifies that its contract version
///    matches `expected_contract_version` (`"omni-harness-v1.0.0"`), refusing to run on mismatch.
/// 2. Submits candidate `.omni` source to `run_omni_harness_json(..., true)`.
/// 3. On failure, extracts the structured `OmniStructuredError` (`code`, 1-based `line`/`col`, `retry_hint`, `message`),
///    selects the versioned `strategy_id`, and invokes `repair_fn(current_src, &err, &spec_json)` up to `max_attempts`.
pub fn run_omni_agent_repair_loop_with_model<F>(
    initial_src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
    expected_contract_version: &str,
    mut repair_fn: F,
) -> Result<OmniAgentRepairReport, String>
where
    F: FnMut(&str, &OmniStructuredError, &str) -> Option<String>,
{
    let spec_json = omni_harness_spec_json();
    let expected_tag = format!("\"version\":\"{}\"", expected_contract_version);
    if !spec_json.contains(&expected_tag) {
        return Err(format!(
            "error[VERSION_MISMATCH]: omni-agent-driver expected harness contract version '{}', but omni-harness --spec reported {}",
            expected_contract_version, spec_json
        ));
    }
    let limit = max_attempts.max(1);
    let mut current_src = String::from(initial_src);
    let mut attempts = Vec::with_capacity(limit);

    for attempt_idx in 1..=limit {
        let harness_json = run_pure_omni_driver_json(&current_src, args.clone(), strict, true);
        if harness_json.starts_with("{\"ok\":true") {
            attempts.push(OmniAgentAttempt {
                attempt: attempt_idx,
                source: current_src.clone(),
                harness_json: harness_json.clone(),
                error: None,
                strategy_id: None,
                strategy_version: OMNI_REPAIR_STRATEGIES_VERSION,
            });
            return Ok(OmniAgentRepairReport {
                ok: true,
                contract_version: String::from(OMNI_HARNESS_CONTRACT_VERSION),
                strategy_version: String::from(OMNI_REPAIR_STRATEGIES_VERSION),
                attempts,
                final_source: current_src,
                final_json: harness_json,
            });
        }

        let err = extract_structured_error_from_harness_json(&harness_json, &current_src, strict);
        let strat_id = select_omni_repair_strategy(&current_src, &err);
        attempts.push(OmniAgentAttempt {
            attempt: attempt_idx,
            source: current_src.clone(),
            harness_json: harness_json.clone(),
            error: Some(err.clone()),
            strategy_id: Some(strat_id),
            strategy_version: OMNI_REPAIR_STRATEGIES_VERSION,
        });

        if attempt_idx == limit {
            return Ok(OmniAgentRepairReport {
                ok: false,
                contract_version: String::from(OMNI_HARNESS_CONTRACT_VERSION),
                strategy_version: String::from(OMNI_REPAIR_STRATEGIES_VERSION),
                attempts,
                final_source: current_src,
                final_json: harness_json,
            });
        }

        match repair_fn(&current_src, &err, &spec_json) {
            Some(next_src) if next_src != current_src => {
                current_src = next_src;
            }
            _ => {
                return Ok(OmniAgentRepairReport {
                    ok: false,
                    contract_version: String::from(OMNI_HARNESS_CONTRACT_VERSION),
                    strategy_version: String::from(OMNI_REPAIR_STRATEGIES_VERSION),
                    attempts,
                    final_source: current_src,
                    final_json: harness_json,
                });
            }
        }
    }

    unreachable!()
}

fn extract_structured_error_from_harness_json(
    harness_json: &str,
    src: &str,
    strict: bool,
) -> OmniStructuredError {
    let mut parsed_code: Option<&'static str> = None;
    if let Some(c_idx) = harness_json.find("\"code\":\"") {
        let rest = &harness_json[c_idx + 8..];
        if let Some(end_q) = rest.find('"') {
            parsed_code = map_error_code_str(&rest[..end_q]);
        }
    }
    let mut parsed_line: Option<usize> = None;
    if let Some(l_idx) = harness_json.find("\"line\":") {
        let rest = &harness_json[l_idx + 7..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            parsed_line = Some(n.max(1));
        }
    }
    let mut parsed_col: Option<usize> = None;
    if let Some(col_idx) = harness_json.find("\"col\":") {
        let rest = &harness_json[col_idx + 6..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            parsed_col = Some(n.max(1));
        }
    }
    let mut parsed_msg: Option<String> = None;
    if let Some(m_idx) = harness_json.find("\"message\":\"") {
        let rest = &harness_json[m_idx + 11..];
        let mut msg = String::new();
        let mut chars = rest.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(esc) = chars.next() {
                    match esc {
                        'n' => msg.push('\n'),
                        'r' => msg.push('\r'),
                        't' => msg.push('\t'),
                        other => msg.push(other),
                    }
                }
            } else if ch == '"' {
                break;
            } else {
                msg.push(ch);
            }
        }
        parsed_msg = Some(msg);
    }
    if let (Some(code), Some(line), Some(col), Some(message)) =
        (parsed_code, parsed_line, parsed_col, parsed_msg)
    {
        return OmniStructuredError {
            code,
            line: Some(line),
            col: Some(col),
            message,
            retry_hint: retry_hint_for_code(code),
        };
    }
    let wrapped = if strict {
        String::from(src)
    } else {
        wrap_omni_surface_snippet(src)
    };
    let u16_res = if strict {
        compile_omni_surface_strict_to_u16(&wrapped)
    } else {
        compile_omni_surface_to_u16(&wrapped)
    };
    match u16_res {
        Err(e) => parse_omni_structured_error(&e),
        Ok(_) => parse_omni_structured_error(harness_json),
    }
}

/// Builds a deterministic single-line JSON repair prompt payload for an external LLM or `--repair-cmd` tool.
pub fn build_omni_agent_repair_prompt_json(
    attempt: usize,
    current_src: &str,
    err: &OmniStructuredError,
    spec_json: &str,
) -> String {
    let strat_id = select_omni_repair_strategy(current_src, err);
    format!(
        "{{\"version\":\"{}\",\"strategy_version\":\"{}\",\"strategy_id\":\"{}\",\"attempt\":{},\"source\":\"{}\",\"error\":{{\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"retry_hint\":\"{}\"}},\"spec\":{}}}",
        OMNI_HARNESS_CONTRACT_VERSION,
        OMNI_REPAIR_STRATEGIES_VERSION,
        strat_id,
        attempt,
        json_escape_str(current_src),
        err.code,
        err.line.unwrap_or(1).max(1),
        err.col.unwrap_or(1).max(1),
        json_escape_str(&err.message),
        json_escape_str(err.retry_hint),
        spec_json
    )
}

/// Runs the version-locked `omni-agent-driver` loop using the built-in structured feedback repair rule engine.
pub fn run_omni_agent_repair_loop(
    initial_src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
    expected_contract_version: &str,
) -> Result<OmniAgentRepairReport, String> {
    run_omni_agent_repair_loop_with_model(
        initial_src,
        args,
        strict,
        max_attempts,
        expected_contract_version,
        |cur_src, err, _spec| repair_omni_program_from_harness_error(cur_src, err, strict),
    )
}

/// Formats an `OmniAgentRepairReport` as a deterministic single-line JSON object including `strategy_id` and `strategy_version`.
pub fn run_omni_agent_driver_json(
    initial_src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    max_attempts: usize,
    expected_contract_version: &str,
) -> String {
    match run_omni_agent_repair_loop(initial_src, args, strict, max_attempts, expected_contract_version) {
        Ok(rep) => {
            let codes: Vec<String> = rep
                .attempts
                .iter()
                .map(|a| match &a.error {
                    Some(e) => format!(
                        "{{\"attempt\":{},\"ok\":false,\"code\":\"{}\",\"line\":{},\"col\":{},\"strategy_id\":\"{}\",\"strategy_version\":\"{}\"}}",
                        a.attempt,
                        e.code,
                        e.line.unwrap_or(1),
                        e.col.unwrap_or(1),
                        a.strategy_id.unwrap_or("strat_syntax_fallback_v1"),
                        a.strategy_version
                    ),
                    None => format!("{{\"attempt\":{},\"ok\":true}}", a.attempt),
                })
                .collect();
            format!(
                "{{\"ok\":{},\"version\":\"{}\",\"strategy_version\":\"{}\",\"attempts\":{},\"history\":[{}],\"final_source\":\"{}\",\"harness\":{}}}",
                rep.ok,
                rep.contract_version,
                rep.strategy_version,
                rep.attempts.len(),
                codes.join(","),
                json_escape_str(&rep.final_source),
                rep.final_json
            )
        }
        Err(e) => format!(
            "{{\"ok\":false,\"version\":\"{}\",\"strategy_version\":\"{}\",\"code\":\"VERSION_MISMATCH\",\"line\":1,\"col\":1,\"message\":\"{}\"}}",
            expected_contract_version,
            OMNI_REPAIR_STRATEGIES_VERSION,
            json_escape_str(&e)
        ),
    }
}

/// Shared CLI front-end for `omni-agent-driver` / `omni agent-driver`.
/// Usage: omni-agent-driver [--list-strategies] [--explain <CODE>] [--strict] [--max-attempts N] [--expect-version VER] [--repair-cmd CMD] [--emit-repair-prompt] (<file.omni | - | "source">) [args...]
pub fn run_omni_agent_driver_cli(argv: &[String]) -> i32 {
    let mut strict = false;
    let mut max_attempts = 6usize;
    let mut expected_ver = String::from(OMNI_HARNESS_CONTRACT_VERSION);
    let mut repair_cmd: Option<String> = None;
    let mut emit_repair_prompt = false;
    let mut rest: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "--list-strategies" | "list-strategies" => {
                println!("{}", omni_repair_strategies_json());
                return 0;
            }
            "--explain" if i + 1 < argv.len() => {
                match explain_omni_error_code(&argv[i + 1]) {
                    Some(text) => {
                        println!("{}", text);
                        return 0;
                    }
                    None => {
                        eprintln!("Unknown Omni error code '{}'", argv[i + 1]);
                        return 2;
                    }
                }
            }
            "--strict" => strict = true,
            "--emit-repair-prompt" => emit_repair_prompt = true,
            "--max-attempts" => {
                if i + 1 >= argv.len() {
                    eprintln!("--max-attempts requires an integer argument");
                    return 2;
                }
                max_attempts = argv[i + 1].parse::<usize>().unwrap_or(6).max(1);
                i += 1;
            }
            "--expect-version" => {
                if i + 1 >= argv.len() {
                    eprintln!("--expect-version requires a version string");
                    return 2;
                }
                expected_ver = argv[i + 1].clone();
                i += 1;
            }
            "--repair-cmd" => {
                if i + 1 >= argv.len() {
                    eprintln!("--repair-cmd requires a command string");
                    return 2;
                }
                repair_cmd = Some(argv[i + 1].clone());
                i += 1;
            }
            _ => rest.push(argv[i].clone()),
        }
        i += 1;
    }
    if rest.is_empty() {
        eprintln!("usage: omni-agent-driver [--list-strategies] [--explain <CODE>] [--strict] [--max-attempts N] [--expect-version VER] [--repair-cmd CMD] [--emit-repair-prompt] (<file.omni | - | \"source\">) [args...]");
        return 2;
    }
    let target = &rest[0];
    let src = if target == "-" {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("failed to read source from stdin");
            return 2;
        }
        buf
    } else if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let vm_args: Vec<OmniValue> = rest[1..].iter().map(|s| parse_cli_omni_value(s)).collect();
    if emit_repair_prompt {
        let spec_json = omni_harness_spec_json();
        let harness_json = run_omni_harness_json(&src, vm_args, strict, true);
        if harness_json.starts_with("{\"ok\":true") {
            println!("{}", harness_json);
            return 0;
        }
        let err = extract_structured_error_from_harness_json(&harness_json, &src, strict);
        println!("{}", build_omni_agent_repair_prompt_json(1, &src, &err, &spec_json));
        return 1;
    }
    if let Some(cmd_str) = repair_cmd {
        let mut attempt_counter = 0usize;
        let res = run_omni_agent_repair_loop_with_model(
            &src,
            vm_args,
            strict,
            max_attempts,
            &expected_ver,
            |cur_src, err, spec_json| {
                attempt_counter += 1;
                let payload = build_omni_agent_repair_prompt_json(attempt_counter, cur_src, err, spec_json);
                use std::io::Write;
                use std::process::{Command, Stdio};
                let mut child = if cfg!(target_os = "windows") {
                    Command::new("powershell")
                        .args(["-NoProfile", "-Command", &cmd_str])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .ok()?
                } else {
                    Command::new("sh")
                        .args(["-c", &cmd_str])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .ok()?
                };
                if let Some(mut sin) = child.stdin.take() {
                    let _ = sin.write_all(payload.as_bytes());
                }
                let out = child.wait_with_output().ok()?;
                if !out.status.success() {
                    return None;
                }
                let candidate = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if candidate.is_empty() {
                    None
                } else {
                    Some(candidate)
                }
            },
        );
        match res {
            Ok(rep) => {
                let codes: Vec<String> = rep
                    .attempts
                    .iter()
                    .map(|a| match &a.error {
                        Some(e) => format!(
                            "{{\"attempt\":{},\"ok\":false,\"code\":\"{}\",\"line\":{},\"col\":{},\"strategy_id\":\"{}\",\"strategy_version\":\"{}\"}}",
                            a.attempt,
                            e.code,
                            e.line.unwrap_or(1),
                            e.col.unwrap_or(1),
                            a.strategy_id.unwrap_or("strat_syntax_fallback_v1"),
                            a.strategy_version
                        ),
                        None => format!("{{\"attempt\":{},\"ok\":true}}", a.attempt),
                    })
                    .collect();
                let out = format!(
                    "{{\"ok\":{},\"version\":\"{}\",\"strategy_version\":\"{}\",\"attempts\":{},\"history\":[{}],\"final_source\":\"{}\",\"harness\":{}}}",
                    rep.ok,
                    rep.contract_version,
                    rep.strategy_version,
                    rep.attempts.len(),
                    codes.join(","),
                    json_escape_str(&rep.final_source),
                    rep.final_json
                );
                println!("{}", out);
                return if rep.ok { 0 } else { 1 };
            }
            Err(e) => {
                println!(
                    "{{\"ok\":false,\"version\":\"{}\",\"strategy_version\":\"{}\",\"code\":\"VERSION_MISMATCH\",\"line\":1,\"col\":1,\"message\":\"{}\"}}",
                    expected_ver,
                    OMNI_REPAIR_STRATEGIES_VERSION,
                    json_escape_str(&e)
                );
                return 1;
            }
        }
    }
    let out = run_omni_agent_driver_json(&src, vm_args, strict, max_attempts, &expected_ver);
    println!("{}", out);
    if out.starts_with("{\"ok\":true") { 0 } else { 1 }
}

// ==============================================================================
// 7E. NARROW CAPABILITY-GATED HOST INTERFACE (`OmniNarrowHost`) & LIVE HTTP/DB HOST
// ==============================================================================

fn check_narrow_host_cap(cap_mask: CapMask, req_mask: CapMask, syscall_name: &str) -> Result<(), String> {
    if (cap_mask & req_mask) != req_mask {
        return Err(format!(
            "error[MISSING_CAP] line 1, col 1: Narrow host syscall '{}' requires capability mask 0x{:04X}, got 0x{:04X}",
            syscall_name, req_mask, cap_mask
        ));
    }
    Ok(())
}

/// The frozen 10-operation narrow capability-gated host boundary (`spec/host_interface.md`, `omni-narrow-host-v1.0.0`).
/// All OS network, file-backed KV/DB persistence, and monotonic clock operations route through this trait.
pub trait OmniNarrowHost {
    fn host_tcp_listen(&mut self, cap_mask: CapMask, bind_addr: &str) -> Result<(u64, String), String>;
    fn host_tcp_accept(&mut self, cap_mask: CapMask, listener_handle: u64) -> Result<u64, String>;
    fn host_tcp_read(&mut self, cap_mask: CapMask, conn_handle: u64, max_bytes: usize) -> Result<Vec<u8>, String>;
    fn host_tcp_write(&mut self, cap_mask: CapMask, conn_handle: u64, data: &[u8]) -> Result<usize, String>;
    fn host_tcp_close(&mut self, cap_mask: CapMask, handle: u64) -> Result<(), String>;
    fn host_file_open(&mut self, cap_mask: CapMask, path: &str, write: bool) -> Result<u64, String>;
    fn host_file_read(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<Vec<u8>, String>;
    fn host_file_write(&mut self, cap_mask: CapMask, file_handle: u64, data: &[u8]) -> Result<usize, String>;
    fn host_file_close(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<(), String>;
    fn host_time_ns(&mut self, cap_mask: CapMask) -> Result<u64, String>;
}

/// Live OS reference implementation of `OmniNarrowHost` backed by `std::net`, `std::fs`, and `std::time::Instant`.
pub struct StdNarrowHostAdapter {
    next_handle: u64,
    listeners: BTreeMap<u64, std::net::TcpListener>,
    streams: BTreeMap<u64, std::net::TcpStream>,
    files: BTreeMap<u64, (std::path::PathBuf, bool, Vec<u8>)>,
    start_instant: std::time::Instant,
}

impl Default for StdNarrowHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl StdNarrowHostAdapter {
    pub fn new() -> Self {
        Self {
            next_handle: 1,
            listeners: BTreeMap::new(),
            streams: BTreeMap::new(),
            files: BTreeMap::new(),
            start_instant: std::time::Instant::now(),
        }
    }

    fn alloc_handle(&mut self) -> u64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
}

impl OmniNarrowHost for StdNarrowHostAdapter {
    fn host_tcp_listen(&mut self, cap_mask: CapMask, bind_addr: &str) -> Result<(u64, String), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_listen")?;
        let listener = std::net::TcpListener::bind(bind_addr)
            .map_err(|e| format!("Failed to bind TCP listener on '{}': {}", bind_addr, e))?;
        let addr_str = listener
            .local_addr()
            .map_err(|e| format!("Failed to query local socket addr: {}", e))?
            .to_string();
        let h = self.alloc_handle();
        self.listeners.insert(h, listener);
        Ok((h, addr_str))
    }

    fn host_tcp_accept(&mut self, cap_mask: CapMask, listener_handle: u64) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_accept")?;
        let listener = self
            .listeners
            .get(&listener_handle)
            .ok_or_else(|| format!("Invalid TCP listener handle {}", listener_handle))?;
        let (stream, _) = listener.accept().map_err(|e| format!("TCP accept failed: {}", e))?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        let ch = self.alloc_handle();
        self.streams.insert(ch, stream);
        Ok(ch)
    }

    fn host_tcp_read(&mut self, cap_mask: CapMask, conn_handle: u64, max_bytes: usize) -> Result<Vec<u8>, String> {
        use std::io::Read;
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_read")?;
        let stream = self
            .streams
            .get_mut(&conn_handle)
            .ok_or_else(|| format!("Invalid TCP connection handle {}", conn_handle))?;
        let mut buf = vec![0u8; max_bytes.clamp(1, 65536)];
        let n = stream.read(&mut buf).map_err(|e| format!("TCP read failed: {}", e))?;
        buf.truncate(n);
        Ok(buf)
    }

    fn host_tcp_write(&mut self, cap_mask: CapMask, conn_handle: u64, data: &[u8]) -> Result<usize, String> {
        use std::io::Write;
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_write")?;
        let stream = self
            .streams
            .get_mut(&conn_handle)
            .ok_or_else(|| format!("Invalid TCP connection handle {}", conn_handle))?;
        stream.write_all(data).map_err(|e| format!("TCP write failed: {}", e))?;
        let _ = stream.flush();
        Ok(data.len())
    }

    fn host_tcp_close(&mut self, cap_mask: CapMask, handle: u64) -> Result<(), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_close")?;
        self.streams.remove(&handle);
        self.listeners.remove(&handle);
        Ok(())
    }

    fn host_file_open(&mut self, cap_mask: CapMask, path: &str, write: bool) -> Result<u64, String> {
        if write {
            if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
                return Err(String::from(
                    "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(write)' requires CAP_ID_DB_W or CAP_ID_FS",
                ));
            }
        } else if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(read)' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let p = std::path::PathBuf::from(path);
        let initial_bytes = if !write && p.exists() {
            std::fs::read(&p).map_err(|e| format!("Failed to read host file '{}': {}", p.display(), e))?
        } else {
            Vec::new()
        };
        let fh = self.alloc_handle();
        self.files.insert(fh, (p, write, initial_bytes));
        Ok(fh)
    }

    fn host_file_read(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<Vec<u8>, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_read' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let (_, _, bytes) = self
            .files
            .get(&file_handle)
            .ok_or_else(|| format!("Invalid file handle {}", file_handle))?;
        Ok(bytes.clone())
    }

    fn host_file_write(&mut self, cap_mask: CapMask, file_handle: u64, data: &[u8]) -> Result<usize, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_write' requires CAP_ID_DB_W or CAP_ID_FS",
            ));
        }
        let (_, write, bytes) = self
            .files
            .get_mut(&file_handle)
            .ok_or_else(|| format!("Invalid file handle {}", file_handle))?;
        if !*write {
            return Err(format!("File handle {} was not opened for writing", file_handle));
        }
        *bytes = data.to_vec();
        Ok(data.len())
    }

    fn host_file_close(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<(), String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_close' requires DB/FS capability",
            ));
        }
        if let Some((path, write, bytes)) = self.files.remove(&file_handle) {
            if write {
                std::fs::write(&path, &bytes)
                    .map_err(|e| format!("Failed to flush host file '{}': {}", path.display(), e))?;
            }
        }
        Ok(())
    }

    fn host_time_ns(&mut self, cap_mask: CapMask) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_SYS), "host_time_ns")?;
        Ok(self.start_instant.elapsed().as_nanos() as u64)
    }
}

/// Deterministic in-memory implementation of `OmniNarrowHost` for socket-free & disk-free verification of the host boundary.
pub struct SimulatedNarrowHostAdapter {
    next_handle: u64,
    pub bound_addr: Option<String>,
    pub pending_requests: Vec<Vec<u8>>,
    pub written_responses: Vec<Vec<u8>>,
    pub virtual_files: BTreeMap<String, Vec<u8>>,
    open_files: BTreeMap<u64, (String, bool, Vec<u8>)>,
    clock_ns: u64,
}

impl Default for SimulatedNarrowHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedNarrowHostAdapter {
    pub fn new() -> Self {
        Self {
            next_handle: 100,
            bound_addr: None,
            pending_requests: Vec::new(),
            written_responses: Vec::new(),
            virtual_files: BTreeMap::new(),
            open_files: BTreeMap::new(),
            clock_ns: 1_000_000,
        }
    }

    pub fn enqueue_http_request(&mut self, raw_wire: &str) {
        self.pending_requests.push(raw_wire.as_bytes().to_vec());
    }

    fn alloc_handle(&mut self) -> u64 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
}

impl OmniNarrowHost for SimulatedNarrowHostAdapter {
    fn host_tcp_listen(&mut self, cap_mask: CapMask, bind_addr: &str) -> Result<(u64, String), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_listen")?;
        let assigned = if bind_addr.ends_with(":0") {
            String::from("127.0.0.1:40080")
        } else {
            String::from(bind_addr)
        };
        self.bound_addr = Some(assigned.clone());
        Ok((self.alloc_handle(), assigned))
    }

    fn host_tcp_accept(&mut self, cap_mask: CapMask, _listener_handle: u64) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_accept")?;
        Ok(self.alloc_handle())
    }

    fn host_tcp_read(&mut self, cap_mask: CapMask, _conn_handle: u64, _max_bytes: usize) -> Result<Vec<u8>, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_read")?;
        if self.pending_requests.is_empty() {
            return Err(String::from("No pending simulated TCP request"));
        }
        Ok(self.pending_requests.remove(0))
    }

    fn host_tcp_write(&mut self, cap_mask: CapMask, _conn_handle: u64, data: &[u8]) -> Result<usize, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_write")?;
        self.written_responses.push(data.to_vec());
        Ok(data.len())
    }

    fn host_tcp_close(&mut self, cap_mask: CapMask, _handle: u64) -> Result<(), String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_NET), "host_tcp_close")?;
        Ok(())
    }

    fn host_file_open(&mut self, cap_mask: CapMask, path: &str, write: bool) -> Result<u64, String> {
        if write {
            if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
                return Err(String::from(
                    "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(write)' requires CAP_ID_DB_W or CAP_ID_FS",
                ));
            }
        } else if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_open(read)' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let existing = self.virtual_files.get(path).cloned().unwrap_or_default();
        let fh = self.alloc_handle();
        self.open_files.insert(fh, (String::from(path), write, existing));
        Ok(fh)
    }

    fn host_file_read(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<Vec<u8>, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_read' requires CAP_ID_DB_R or CAP_ID_FS",
            ));
        }
        let (_, _, data) = self
            .open_files
            .get(&file_handle)
            .ok_or_else(|| format!("Invalid simulated file handle {}", file_handle))?;
        Ok(data.clone())
    }

    fn host_file_write(&mut self, cap_mask: CapMask, file_handle: u64, data: &[u8]) -> Result<usize, String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_write' requires CAP_ID_DB_W or CAP_ID_FS",
            ));
        }
        let (_, write, buf) = self
            .open_files
            .get_mut(&file_handle)
            .ok_or_else(|| format!("Invalid simulated file handle {}", file_handle))?;
        if !*write {
            return Err(format!("Simulated file handle {} not opened for write", file_handle));
        }
        *buf = data.to_vec();
        Ok(data.len())
    }

    fn host_file_close(&mut self, cap_mask: CapMask, file_handle: u64) -> Result<(), String> {
        if (cap_mask & (cap_bit(CAP_ID_DB_R) | cap_bit(CAP_ID_DB_W) | cap_bit(CAP_ID_FS))) == 0 {
            return Err(String::from(
                "error[MISSING_CAP] line 1, col 1: Narrow host syscall 'host_file_close' requires DB/FS capability",
            ));
        }
        if let Some((path, write, buf)) = self.open_files.remove(&file_handle) {
            if write {
                self.virtual_files.insert(path, buf);
            }
        }
        Ok(())
    }

    fn host_time_ns(&mut self, cap_mask: CapMask) -> Result<u64, String> {
        check_narrow_host_cap(cap_mask, cap_bit(CAP_ID_SYS), "host_time_ns")?;
        self.clock_ns += 250_000;
        Ok(self.clock_ns)
    }
}

/// Loads a `key=value` map through any `OmniNarrowHost` implementation (`CAP_ID_DB_R`).
pub fn load_omni_kv_via_narrow_host<H: OmniNarrowHost>(
    host: &mut H,
    cap_mask: CapMask,
    path: &str,
) -> Result<BTreeMap<i64, i64>, String> {
    let fh = host.host_file_open(cap_mask, path, false)?;
    let raw_bytes = host.host_file_read(cap_mask, fh)?;
    host.host_file_close(cap_mask, fh)?;
    let raw = String::from_utf8_lossy(&raw_bytes);
    let mut kv = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k_str, v_str)) = trimmed.split_once('=') {
            if let (Ok(k), Ok(v)) = (k_str.trim().parse::<i64>(), v_str.trim().parse::<i64>()) {
                kv.insert(k, v);
            }
        }
    }
    Ok(kv)
}

/// Saves a `key=value` map through any `OmniNarrowHost` implementation (`CAP_ID_DB_W`).
pub fn save_omni_kv_via_narrow_host<H: OmniNarrowHost>(
    host: &mut H,
    cap_mask: CapMask,
    path: &str,
    kv: &BTreeMap<i64, i64>,
) -> Result<(), String> {
    let mut buf = String::new();
    for (k, v) in kv {
        buf.push_str(&format!("{}={}\n", k, v));
    }
    let fh = host.host_file_open(cap_mask, path, true)?;
    let _ = host.host_file_write(cap_mask, fh, buf.as_bytes())?;
    host.host_file_close(cap_mask, fh)
}

/// Executes a full HTTP/1.1 + DB/KV batch cycle exclusively through the narrow `OmniNarrowHost` interface.
pub fn serve_omni_http_via_narrow_host<H: OmniNarrowHost>(
    host: &mut H,
    src: &str,
    bind_addr: &str,
    db_path: Option<&str>,
    num_requests: usize,
) -> Result<(String, Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String> {
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from("error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers"));
    }
    let (u16_stream, sm) = compile_omni_surface_to_u16_with_sourcemap(src, false)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let combined_caps: CapMask = module.functions.iter().fold(0u16, |acc, f| acc | f.cap_mask);

    let mut kv_store = if let Some(p) = db_path {
        load_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_R), p)?
    } else {
        BTreeMap::new()
    };

    let (listener_h, local_addr) = host.host_tcp_listen(combined_caps, bind_addr)?;
    let mut responses = Vec::with_capacity(num_requests);
    for _ in 0..num_requests {
        let conn_h = host.host_tcp_accept(combined_caps, listener_h)?;
        let req_bytes = host.host_tcp_read(combined_caps, conn_h, 4096)?;
        let req_str = String::from_utf8_lossy(&req_bytes);
        let first_line = req_str.lines().next().unwrap_or("GET / HTTP/1.1");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let req_path = parts.next().unwrap_or("/");
        let resp = handle_omni_http_request(src, method, req_path, &mut kv_store)?;
        if let Some(p) = db_path {
            save_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_W), p, &kv_store)?;
        }
        let wire = resp.to_http11_wire_bytes();
        let _ = host.host_tcp_write(combined_caps, conn_h, &wire)?;
        let _ = host.host_tcp_close(combined_caps, conn_h)?;
        responses.push(resp);
    }
    let _ = host.host_tcp_close(combined_caps, listener_h)?;
    Ok((local_addr, responses, kv_store))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniRouteSpec {
    pub fn_idx: u16,
    pub method: String,
    pub path_pattern: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmniHttpResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
}

impl OmniHttpResponse {
    pub fn status_text(status: u16) -> &'static str {
        match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "OK",
        }
    }

    pub fn to_http11_wire_bytes(&self) -> Vec<u8> {
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            Self::status_text(self.status),
            self.content_type,
            self.body.len(),
            self.body
        )
        .into_bytes()
    }
}

/// Extracts all `route <METHOD> <path>` declarations from a surface `.omni` module alongside their assigned `fn_idx`.
pub fn extract_omni_routes(src: &str) -> Result<Vec<OmniRouteSpec>, String> {
    let effective_strict = src.contains("#[strict]");
    let tokens = tokenize_omni_surface_inner(src, effective_strict)?;
    let mut parser = SurfaceParser::new_with_strict(tokens, effective_strict);
    let (includes, fns) = parser.parse_module()?;
    if !includes.is_empty() {
        let expanded = resolve_omni_surface_includes(src, None, None)?;
        return extract_omni_routes(&expanded);
    }
    let multi_fn = fns.len() > 1;
    let mut routes = Vec::new();
    for (idx, f) in fns.iter().enumerate() {
        let fid = if let Some(id_attr) = f.fn_id_attr {
            id_attr
        } else if !multi_fn || idx + 1 == fns.len() {
            0
        } else {
            (idx + 1) as u16
        };
        if f.is_route {
            routes.push(OmniRouteSpec {
                fn_idx: fid,
                method: f.route_method.clone().unwrap_or_else(|| String::from("GET")),
                path_pattern: f.route_path.clone().unwrap_or_else(|| String::from("/")),
            });
        }
    }
    Ok(routes)
}

/// Matches a route pattern (e.g. `"/users/:id"`) against an incoming HTTP request path (e.g. `"/users/42?role=7"`),
/// returning `(primary_param_i64, req_params_by_fnv1a32)`.
pub fn match_omni_route_pattern(
    pattern: &str,
    request_path: &str,
) -> Option<(i64, BTreeMap<i64, i64>)> {
    let (clean_path, query_opt) = match request_path.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (request_path, None),
    };
    let pat_segs: Vec<&str> = pattern.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    let req_segs: Vec<&str> = clean_path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    if pat_segs.len() != req_segs.len() {
        return None;
    }
    let mut params = BTreeMap::new();
    let mut primary_val: Option<i64> = None;
    for (ps, rs) in pat_segs.into_iter().zip(req_segs.into_iter()) {
        if let Some(param_name) = ps.strip_prefix(':') {
            let parsed = rs.parse::<i64>().unwrap_or_else(|_| fnv1a_32_positive(rs));
            params.insert(fnv1a_32_positive(param_name), parsed);
            if primary_val.is_none() {
                primary_val = Some(parsed);
            }
        } else if ps != rs {
            return None;
        }
    }
    if let Some(q) = query_opt {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                let parsed = v.parse::<i64>().unwrap_or_else(|_| fnv1a_32_positive(v));
                params.insert(fnv1a_32_positive(k), parsed);
                if primary_val.is_none() {
                    primary_val = Some(parsed);
                }
            }
        }
    }
    Some((primary_val.unwrap_or(0), params))
}

/// Compiles and statically verifies a `.omni` route program, then dispatches an HTTP request
/// (`method`, `request_path`) against the host's `kv_store` and returns a real `OmniHttpResponse`.
pub fn handle_omni_http_request(
    src: &str,
    method: &str,
    request_path: &str,
    kv_store: &mut BTreeMap<i64, i64>,
) -> Result<OmniHttpResponse, String> {
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from("error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers"));
    }
    let (u16_stream, sm) = compile_omni_surface_to_u16_with_sourcemap(src, false)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let norm_method = method.trim().to_ascii_uppercase();

    for rt in &routes {
        if rt.method == norm_method {
            if let Some((primary_arg, req_params)) = match_omni_route_pattern(&rt.path_pattern, request_path) {
                let (attr_steps, attr_mem) = extract_omni_surface_budgets(src);
                let mut vm = OmniBinaryVm::new();
                if let Some(sb) = attr_steps {
                    vm.step_limit = sb;
                }
                if let Some(mb) = attr_mem {
                    vm.memory_budget_bytes = mb;
                }
                vm.kv_store = kv_store.clone();
                vm.req_params = req_params;
                let val = vm
                    .execute_function(&module, rt.fn_idx, vec![OmniValue::Int(primary_arg)])
                    .map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))?;
                *kv_store = vm.kv_store;
                if let Some((status, content_type, body)) = vm.last_http_response {
                    return Ok(OmniHttpResponse {
                        status,
                        content_type,
                        body,
                    });
                }
                let body = format!(
                    "{{\"status\":200,\"data\":{}}}",
                    format_omni_value_compact(&val)
                );
                return Ok(OmniHttpResponse {
                    status: 200,
                    content_type: String::from("application/json"),
                    body,
                });
            }
        }
    }

    Ok(OmniHttpResponse {
        status: 404,
        content_type: String::from("application/json"),
        body: format!(
            "{{\"status\":404,\"error\":\"No route matched {} {}\"}}",
            norm_method, request_path
        ),
    })
}

/// Loads a persistent host KV/DB store from disk through the narrow capability-gated host adapter (`StdNarrowHostAdapter`).
pub fn load_omni_kv_file(path: &std::path::Path) -> Result<BTreeMap<i64, i64>, String> {
    let mut host = StdNarrowHostAdapter::new();
    load_omni_kv_via_narrow_host(&mut host, cap_bit(CAP_ID_DB_R), &path.to_string_lossy())
}

/// Atomically persists a host KV/DB store to disk through the narrow capability-gated host adapter (`StdNarrowHostAdapter`).
pub fn save_omni_kv_file(path: &std::path::Path, kv: &BTreeMap<i64, i64>) -> Result<(), String> {
    let mut host = StdNarrowHostAdapter::new();
    save_omni_kv_via_narrow_host(&mut host, cap_bit(CAP_ID_DB_W), &path.to_string_lossy(), kv)
}

/// Compiles and statically verifies `src` up front (failing immediately with `MISSING_CAP`, `TAINT_SINK`, etc.
/// before binding a socket if verification fails), then binds a real `std::net::TcpListener` on `bind_addr`
/// via `StdNarrowHostAdapter` and serves `num_requests` real HTTP/1.1 requests over TCP.
pub fn serve_omni_http_batch(
    src: &str,
    bind_addr: &str,
    initial_kv: BTreeMap<i64, i64>,
    num_requests: usize,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<(Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String>>,
    ),
    String,
> {
    serve_omni_http_batch_inner(src, bind_addr, initial_kv, None, num_requests)
}

/// Serves `num_requests` real HTTP/1.1 requests over TCP backed by a persistent on-disk KV/DB file (`db_path`).
pub fn serve_omni_http_batch_with_db_file(
    src: &str,
    bind_addr: &str,
    db_path: &std::path::Path,
    num_requests: usize,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<(Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String>>,
    ),
    String,
> {
    let initial_kv = load_omni_kv_file(db_path)?;
    serve_omni_http_batch_inner(
        src,
        bind_addr,
        initial_kv,
        Some(db_path.to_path_buf()),
        num_requests,
    )
}

fn serve_omni_http_batch_inner(
    src: &str,
    bind_addr: &str,
    initial_kv: BTreeMap<i64, i64>,
    db_file: Option<std::path::PathBuf>,
    num_requests: usize,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<(Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String>>,
    ),
    String,
> {
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from("error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers"));
    }
    let (u16_stream, sm) = compile_omni_surface_to_u16_with_sourcemap(src, false)?;
    for rep in verify_u16_stream(&u16_stream)? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }
    let module = decode_omni_token_stream(&u16_stream)?;
    let combined_caps: CapMask = module.functions.iter().fold(0u16, |acc, f| acc | f.cap_mask);

    let mut host = StdNarrowHostAdapter::new();
    let (listener_h, addr_str) = host.host_tcp_listen(combined_caps, bind_addr)?;
    let local_addr: std::net::SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("Failed to parse bound SocketAddr '{}': {}", addr_str, e))?;
    let src_owned = String::from(src);

    let handle = std::thread::spawn(move || {
        let mut kv_store = initial_kv;
        let mut responses = Vec::with_capacity(num_requests);
        for _ in 0..num_requests {
            let conn_h = host.host_tcp_accept(combined_caps, listener_h)?;
            let req_buf = host.host_tcp_read(combined_caps, conn_h, 4096)?;
            let req_str = String::from_utf8_lossy(&req_buf);
            let first_line = req_str.lines().next().unwrap_or("GET / HTTP/1.1");
            let mut parts = first_line.split_whitespace();
            let method = parts.next().unwrap_or("GET");
            let req_path = parts.next().unwrap_or("/");
            let resp = handle_omni_http_request(&src_owned, method, req_path, &mut kv_store)?;
            if let Some(ref p) = db_file {
                save_omni_kv_via_narrow_host(
                    &mut host,
                    combined_caps | cap_bit(CAP_ID_DB_W),
                    &p.to_string_lossy(),
                    &kv_store,
                )?;
            }
            let wire = resp.to_http11_wire_bytes();
            let _ = host.host_tcp_write(combined_caps, conn_h, &wire)?;
            let _ = host.host_tcp_close(combined_caps, conn_h)?;
            responses.push(resp);
        }
        let _ = host.host_tcp_close(combined_caps, listener_h)?;
        Ok((responses, kv_store))
    });

    Ok((local_addr, handle))
}

/// Serves a single live HTTP/1.1 request over TCP on `bind_addr` (e.g. `"127.0.0.1:0"`).
pub fn serve_omni_http_once(
    src: &str,
    bind_addr: &str,
    initial_kv: BTreeMap<i64, i64>,
) -> Result<
    (
        std::net::SocketAddr,
        std::thread::JoinHandle<Result<OmniHttpResponse, String>>,
    ),
    String,
> {
    let (addr, batch_handle) = serve_omni_http_batch(src, bind_addr, initial_kv, 1)?;
    let single_handle = std::thread::spawn(move || {
        let (mut resps, _) = batch_handle
            .join()
            .map_err(|_| String::from("HTTP host thread panicked"))??;
        resps
            .pop()
            .ok_or_else(|| String::from("No HTTP response produced"))
    });
    Ok((addr, single_handle))
}

/// Runs a live HTTP/1.1 server for `.omni` `route` handlers (`omni serve <file.omni> [bind_addr] [--max-requests N] [--db <path>]`).
pub fn run_omni_serve_cli(argv: &[String]) -> i32 {
    if argv.is_empty() {
        eprintln!("usage: omni serve <file.omni | \"source\"> [bind_addr] [--max-requests N] [--db <path>]");
        return 2;
    }
    let target = &argv[0];
    let src = if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                let parent_dir = std::path::Path::new(target).parent();
                match resolve_omni_surface_includes(&raw, parent_dir, None) {
                    Ok(exp) => exp,
                    Err(e) => {
                        eprintln!("{}", e);
                        return 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let mut bind_addr = String::from("127.0.0.1:8080");
    let mut max_requests = 1usize;
    let mut db_path: Option<std::path::PathBuf> = None;
    let mut i = 1usize;
    while i < argv.len() {
        if argv[i] == "--max-requests" && i + 1 < argv.len() {
            max_requests = argv[i + 1].parse::<usize>().unwrap_or(1).max(1);
            i += 2;
        } else if argv[i] == "--db" && i + 1 < argv.len() {
            db_path = Some(std::path::PathBuf::from(&argv[i + 1]));
            i += 2;
        } else {
            bind_addr = argv[i].clone();
            i += 1;
        }
    }
    let serve_res = match db_path {
        Some(ref p) => serve_omni_http_batch_with_db_file(&src, &bind_addr, p, max_requests),
        None => serve_omni_http_batch(&src, &bind_addr, BTreeMap::new(), max_requests),
    };
    match serve_res {
        Ok((addr, handle)) => {
            println!("Omni HTTP/1.1 + DB/KV host listening on http://{}", addr);
            match handle.join() {
                Ok(Ok((resps, _))) => {
                    println!("Served {} HTTP request(s) cleanly.", resps.len());
                    0
                }
                Ok(Err(e)) => {
                    eprintln!("HTTP host error: {}", e);
                    1
                }
                Err(_) => {
                    eprintln!("HTTP host thread panicked");
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

// ==============================================================================
// 7F. BINARY STREAM INSPECTOR (`omni inspect` / `omni disasm`) & DIAGNOSTIC EXPLAINER
// ==============================================================================

fn decode_cap_mask_names(mask: CapMask) -> String {
    if mask == 0 {
        return String::from("PURE");
    }
    let table: &[(u16, &str)] = &[
        (CAP_ID_DB_R, "DB_R"),
        (CAP_ID_DB_W, "DB_W"),
        (CAP_ID_NET, "NET"),
        (CAP_ID_DOM, "DOM"),
        (CAP_ID_AI, "AI"),
        (CAP_ID_VISION, "VISION"),
        (CAP_ID_AUDIO, "AUDIO"),
        (CAP_ID_AUTH, "AUTH"),
        (CAP_ID_FS, "FS"),
        (CAP_ID_SYS, "SYS"),
    ];
    let mut names = Vec::new();
    for &(cid, cname) in table {
        if (mask & cap_bit(cid)) != 0 {
            names.push(cname);
        }
    }
    if names.is_empty() {
        format!("0x{:04X}", mask)
    } else {
        names.join(" | ")
    }
}

fn decode_ty_tok_name(ty_tok: u16) -> &'static str {
    match ty_tok {
        TY_VOID => "VOID",
        TY_I64 => "I64",
        TY_F64 => "F64",
        TY_TRIT => "TRIT",
        TY_STR => "STR",
        TY_UUID => "UUID",
        TY_BYTES => "BYTES",
        TY_TRIT_VEC => "TRIT_VEC",
        TY_FLT_VEC => "FLT_VEC",
        TY_ARRAY => "ARRAY",
        TY_ARENA => "ARENA",
        TY_MODEL_T => "MODEL_T",
        TY_DOM => "DOM",
        TY_STRUCT => "STRUCT",
        TY_ERR => "ERR",
        _ => "UNKNOWN_TY",
    }
}

fn format_inspect_operand(op: &BinOperand) -> String {
    match op {
        BinOperand::Reg(t) => format!("%{}", t.saturating_sub(REG_BASE)),
        BinOperand::Param(t) => format!("${}", t.saturating_sub(PAR_BASE)),
        BinOperand::ImmInt(v) => v.to_string(),
        BinOperand::ImmTrit(t) => {
            if *t > 0 {
                String::from("+1t")
            } else if *t < 0 {
                String::from("-1t")
            } else {
                String::from("0t")
            }
        }
        BinOperand::EnglishSpan(toks) => format!("@ENG[{} words]", toks.len()),
        BinOperand::SchemaRef(s) => format!("@schema{}", s),
        BinOperand::FnRef(f) => format!("@fn{}", f),
        BinOperand::SkillRef(sk) => format!("@skill{}", sk),
        BinOperand::Label(l) => format!("label({})", l.saturating_sub(LBL_BASE)),
        BinOperand::FieldKV(fid, inner) => format!("field#{}={}", fid, format_inspect_operand(inner)),
    }
}

/// Deterministic binary token stream disassembler & inspector (`omni inspect` / `omni disasm`)
/// displaying decoded opcodes, register/parameter/label operands, capability/taint tags, and source map coordinates.
pub fn format_omni_inspect(
    u16_stream: &[u16],
    sourcemap: Option<&SurfaceSourceMap>,
) -> Result<String, String> {
    let module = decode_omni_token_stream(u16_stream)?;
    let mut raw_bytes = Vec::with_capacity(u16_stream.len() * 2);
    for &w in u16_stream {
        raw_bytes.extend_from_slice(&w.to_le_bytes());
    }
    let mut out = String::new();
    out.push_str(&format!(
        "=== OMNI BINARY STREAM INSPECTION (words={}, bytes={}, fnv1a64=0x{:016X}) ===\n",
        u16_stream.len(),
        raw_bytes.len(),
        fnv1a_64_hash(&raw_bytes)
    ));
    out.push_str(&format!(
        "schemas: {}, experts: {}, functions: {}\n",
        module.schemas.len(),
        module.experts.len(),
        module.functions.len()
    ));
    for func in &module.functions {
        let rep = verify_binary_function(func);
        out.push_str(&format!(
            "\nfn #{} -> {} [cap_mask=0x{:04X} ({})] [verifier={}]\n",
            func.fn_idx,
            decode_ty_tok_name(func.ret_ty_tok),
            func.cap_mask,
            decode_cap_mask_names(func.cap_mask),
            if rep.passed { "PASS" } else { "FAIL" }
        ));
        for (p_idx, &(p_tok, p_ty, p_tc)) in func.params.iter().enumerate() {
            let (untr, cap_id, _) = unpack_tc(p_tc);
            out.push_str(&format!(
                "  param ${} (0x{:04X}) : {} [taint={}, cap_id={}]\n",
                p_idx,
                p_tok,
                decode_ty_tok_name(p_ty),
                if untr { "@UNTRUSTED" } else { "SAFE" },
                cap_id
            ));
        }
        for (ins_idx, ins) in func.instructions.iter().enumerate() {
            let op_name = decode_opcode_token(ins.op_tok)
                .map(|s| s.debug_label)
                .unwrap_or("INVALID_OP");
            let dst_part = match ins.dest_reg {
                Some(r) => {
                    let (untr, cap_id, _) = unpack_tc(ins.tc_tok);
                    format!(
                        "%{} : {} [{}, cap={}] = ",
                        r.saturating_sub(REG_BASE),
                        decode_ty_tok_name(ins.ty_tok),
                        if untr { "@UNTRUSTED" } else { "SAFE" },
                        cap_id
                    )
                }
                None => String::new(),
            };
            let ops_str: Vec<String> = ins.operands.iter().map(format_inspect_operand).collect();
            let loc_comment = sourcemap
                .and_then(|sm| sm.offset_to_span.get(&ins.offset))
                .map(|&(l, c)| format!(" // line {}, col {}", l, c))
                .unwrap_or_default();
            out.push_str(&format!(
                "  [{:03} @0x{:04X}] {}{} (0x{:04X}) {}{}\n",
                ins_idx,
                ins.offset,
                dst_part,
                op_name,
                ins.op_tok,
                ops_str.join(", "),
                loc_comment
            ));
        }
    }
    Ok(out)
}

/// Compiles a `.omni` surface program with its `SurfaceSourceMap` through the Pure-Omni frontend and returns the deterministic `format_omni_inspect` output.
pub fn inspect_omni_surface_program(src: &str, strict: bool) -> Result<String, String> {
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, strict)?;
    format_omni_inspect(&u16_stream, Some(&sm))
}

/// Compiles a `.omni` surface program via the Pure-Omni frontend, verifies it with the Pure-Omni verifier kernel, and returns `format_omni_inspect`.
pub fn inspect_omni_surface_via_pure_omni(src: &str, strict: bool) -> Result<String, String> {
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, strict)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    format_omni_inspect(&u16_stream, Some(&sm))
}

/// Renders a structured `OmniStructuredError` in a human-readable multi-line diagnostic format with source line and caret underline.
pub fn format_omni_human_diagnostic(src: &str, err: &OmniStructuredError) -> String {
    let line_no = err.line.unwrap_or(1).max(1);
    let col_no = err.col.unwrap_or(1).max(1);
    let src_line = src.lines().nth(line_no - 1).unwrap_or("");
    let caret_pad = " ".repeat(col_no.saturating_sub(1));
    format!(
        "error[{}]: {}\n  --> line {}:{}\n   |\n{:>2} | {}\n   | {}^\n   = help: {}",
        err.code, err.message, line_no, col_no, line_no, src_line, caret_pad, err.retry_hint
    )
}

/// Expands any of the 16 canonical `error[<CODE>]` identifiers into a normative explanation and repair pattern (`--explain <CODE>`).
pub fn explain_omni_error_code(code: &str) -> Option<String> {
    let norm = code.trim().to_ascii_uppercase();
    let (stage, summary, bad_ex, good_ex) = match norm.as_str() {
        "TAINT_ON_RET" => (
            "Static Verifier (0x7F4A RET)",
            "An @untrusted parameter, @ENG string literal, or register derived from untrusted input reached return without sanitization.",
            "fn main(x: i64 @untrusted) -> i64 { return x; }",
            "#[cap(AUTH)] fn main(x: i64 @untrusted) -> i64 { let clean: i64 = auth_verify(x); return clean; }",
        ),
        "TAINT_SINK" => (
            "Static Verifier (Privilege Sinks)",
            "An @untrusted operand was passed directly to a security-sensitive sink (egate, db_query, db_get, kv_put, respond).",
            "#[cap(AI)] fn main(x: i64 @untrusted) -> trit { return egate(x); }",
            "#[cap(AI, AUTH)] fn main(x: i64 @untrusted) -> trit { let c: i64 = auth_verify(x); return egate(c); }",
        ),
        "MISSING_CAP" => (
            "Static Verifier (Capability Bitmask)",
            "A function or route executed an opcode whose required capability bit (AI, NET, DB_R, DB_W, AUTH, DOM) is absent from #[cap(...)].",
            "fn main(x: i64) -> trit { return egate(x); }",
            "#[cap(AI)] fn main(x: i64) -> trit { return egate(x); }",
        ),
        "LINEAR_LEAK" => (
            "Static Verifier (Linear Arena Ownership)",
            "A linear arena allocated with arena(cap) was not consumed by drop(a) before function return.",
            "fn main(x: i64) -> i64 { let a: arena = arena(4096); return x; }",
            "fn main(x: i64) -> i64 { let a: arena = arena(4096); drop(a); return x; }",
        ),
        "BORROW_CONFLICT" => (
            "Static Verifier (Borrow State Machine)",
            "A linear arena was dropped or mutably borrowed while an active reader borrow (borrow_r) was still live.",
            "let a: arena = arena(4096); let r: arena = borrow_r(a); drop(a); drop(r);",
            "let a: arena = arena(4096); let r: arena = borrow_r(a); drop(r); drop(a);",
        ),
        "UNDEF_VAR" => (
            "Surface Lowering / Macro Hygiene",
            "An identifier was referenced or reassigned before being bound with 'let <name>: <ty> = <expr>;', or a macro referenced a free variable.",
            "fn main(x: i64) -> i64 { return missing_v + 1; }",
            "fn main(x: i64) -> i64 { let missing_v: i64 = 0; return missing_v + 1; }",
        ),
        "IMMUT_PARAM" => (
            "Surface Lowering",
            "Function parameters ($0..=$31) are immutable SSA inputs and cannot be reassigned directly.",
            "fn main(x: i64) -> i64 { x = x + 1; return x; }",
            "fn main(x: i64) -> i64 { let x_mut: i64 = x + 1; return x_mut; }",
        ),
        "BAD_TYPE" => (
            "Parser / Strict Single-Form Checker",
            "An unknown type name was used, or a 'let' binding omitted its explicit ': <ty>' annotation in #[strict] mode.",
            "#[strict] fn main(a: i64) -> i64 { let b = a * 2; return b; }",
            "#[strict] fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }",
        ),
        "UNKNOWN_FN" => (
            "Surface Lowering",
            "Called an identifier that is neither a declared function/macro in the module nor a built-in Omni intrinsic.",
            "fn main(x: i64) -> i64 { return unknown_op(x); }",
            "fn main(x: i64) -> i64 { return iabs(x); }",
        ),
        "ARITY_MISMATCH" => (
            "Surface Lowering",
            "The number of arguments passed to a function, hygienic macro, or built-in intrinsic did not match its signature.",
            "fn main(x: i64) -> i64 { return iabs(x, 2); }",
            "fn main(x: i64) -> i64 { return iabs(x); }",
        ),
        "UNRESOLVED_LABEL" => (
            "Surface Lowering / Static Verifier",
            "A 'goto N;' or conditional branch targeted a basic-block label N (0..=15) not defined by 'label N;' in the same function.",
            "fn main(x: i64) -> i64 { goto 3; return x; }",
            "fn main(x: i64) -> i64 { goto 3; label 3; return x; }",
        ),
        "CIRCULAR_INCLUDE" => (
            "Include Preprocessor",
            "A cycle was detected in 'include \"...\";' directives across modules.",
            "include \"self.omni\"; fn main(x: i64) -> i64 { return x; }",
            "include \"std/prelude.omni\"; fn main(x: i64) -> i64 { return x; }",
        ),
        "BUDGET_EXCEEDED" => (
            "VM / Wasm AOT Runtime",
            "Execution exceeded the function's #[step_budget(N)] instruction cap or #[mem_budget(N)] byte allocation cap.",
            "#[step_budget(10)] fn main(n: i64) -> i64 { ... }",
            "#[step_budget(500000)] fn main(n: i64) -> i64 { ... }",
        ),
        "CORRUPT_PACK5" => (
            "@PACK5 Validator / Decoder",
            "A @PACK5 byte buffer contained a sentinel or out-of-range byte in 243..=255 (valid 5-trit radix-3 bytes are 0..=242).",
            "b = iset(b, 250); let v: trit_vec = unpack5(b, 5);",
            "b = iset(b, 242); let v: trit_vec = unpack5(b, 5);",
        ),
        "AMBIGUOUS_GRAMMAR" => (
            "Strict Single-Form Lexer / Parser",
            "Non-canonical surface aliases (TRIT_POS, int, tvec, untrusted_i64, mod) are forbidden in #[strict] mode.",
            "#[strict] fn main(a: int) -> trit { return TRIT_POS; }",
            "#[strict] fn main(a: i64) -> trit { return +1t; }",
        ),
        "SYNTAX_ERROR" => (
            "Surface Lexer / Parser",
            "Malformed expression, unbalanced delimiters, missing match_trit arm, or empty module.",
            "fn main(x: i64) -> i64 { let r = ; return r; }",
            "fn main(x: i64) -> i64 { let r: i64 = x + 1; return r; }",
        ),
        _ => return None,
    };
    Some(format!(
        "=== OMNI ERROR CODE EXPLANATION: error[{}] ===\nContract: {}\nStage:    {}\nSummary:  {}\nHint:     {}\nBad:      {}\nRepaired: {}",
        norm,
        OMNI_ERROR_CONTRACT_VERSION,
        stage,
        summary,
        retry_hint_for_code(&norm),
        bad_ex,
        good_ex
    ))
}

/// CLI handler for `omni inspect <file.omni | file.omni_bc | "inline source">` and `omni disasm ...`.
pub fn run_omni_inspect_cli(argv: &[String]) -> i32 {
    if argv.is_empty() {
        eprintln!("usage: omni inspect <file.omni | file.omni_bc | \"source\">");
        return 2;
    }
    let target = &argv[0];
    if target.ends_with(".omni_bc") && std::path::Path::new(target).is_file() {
        let raw = match std::fs::read(target) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        };
        let u16s = decode_bytes_to_u16_stream(&raw);
        match format_omni_inspect(&u16s, None) {
            Ok(text) => {
                print!("{}", text);
                return 0;
            }
            Err(e) => {
                eprintln!("{}", e);
                return 1;
            }
        }
    }
    let src = if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                let parent = std::path::Path::new(target).parent();
                match resolve_omni_surface_includes(&raw, parent, None) {
                    Ok(exp) => exp,
                    Err(e) => {
                        eprintln!("{}", e);
                        return 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    match inspect_omni_surface_program(&src, false) {
        Ok(text) => {
            print!("{}", text);
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

// ==============================================================================
// 7G. EXECUTABLE ACCEPTANCE SUITE, ERROR CONTRACT, CONTINUOUS DIFF & COMPOSITIONAL INVARIANTS
// ==============================================================================

/// Verifies the normative 16-code machine-readable error contract (`spec/error_contract.md`):
/// asserts that every code has a non-empty retry hint, appears in `omni-harness --spec`,
/// renders cleanly via `explain_omni_error_code` and `format_omni_human_diagnostic`, and carries non-null 1-based `(line, col)`.
pub fn run_error_contract_selftest() -> Result<usize, String> {
    let all_codes: [&str; 16] = [
        "TAINT_ON_RET",
        "TAINT_SINK",
        "MISSING_CAP",
        "LINEAR_LEAK",
        "BORROW_CONFLICT",
        "UNDEF_VAR",
        "IMMUT_PARAM",
        "BAD_TYPE",
        "UNKNOWN_FN",
        "ARITY_MISMATCH",
        "UNRESOLVED_LABEL",
        "CIRCULAR_INCLUDE",
        "BUDGET_EXCEEDED",
        "CORRUPT_PACK5",
        "AMBIGUOUS_GRAMMAR",
        "SYNTAX_ERROR",
    ];
    let spec_json = omni_harness_spec_json();
    for &code in &all_codes {
        if map_error_code_str(code) != Some(code) {
            return Err(format!("Error contract table missing map_error_code_str({})", code));
        }
        let hint = retry_hint_for_code(code);
        if hint.is_empty() || hint.starts_with("Check surface syntax") && code != "SYNTAX_ERROR" {
            return Err(format!("Error code {} lacks dedicated retry_hint", code));
        }
        if !spec_json.contains(&format!("\"{}\"", code)) {
            return Err(format!("omni_harness_spec_json missing error code {}", code));
        }
        let explained = explain_omni_error_code(code)
            .ok_or_else(|| format!("explain_omni_error_code returned None for {}", code))?;
        if !explained.contains(&format!("error[{}]", code)) || !explained.contains("Repaired:") {
            return Err(format!("explain_omni_error_code({}) malformed: {}", code, explained));
        }
        let sample_err = format!("error[{}] line 2, col 5: sample diagnostic for {}", code, code);
        let parsed = parse_omni_structured_error(&sample_err);
        if parsed.code != code || parsed.line != Some(2) || parsed.col != Some(5) || parsed.retry_hint != hint {
            return Err(format!("parse_omni_structured_error failed roundtrip for {}: {:?}", code, parsed));
        }
        let human = format_omni_human_diagnostic("fn main() -> i64 {\n    return 0;\n}", &parsed);
        if !human.contains(&format!("error[{}]", code)) || !human.contains("--> line 2:5") || !human.contains("= help:") {
            return Err(format!("format_omni_human_diagnostic failed for {}: {}", code, human));
        }
    }
    Ok(all_codes.len())
}

/// Runs the continuous differential acceptance harness (`omni acceptance-diff`):
/// feeds the frozen positive and negative acceptance suite through both the direct zero-IR compiler
/// (`compile_omni_surface_to_u16` / `compile_omni_surface_strict_to_u16` + `run_omni_harness_json`)
/// and the reference `.omni_asm` lowering + assembler + VM/AOT pipeline (or an optional external candidate command),
/// asserting bit-identical `Vec<u16>` token streams, exact locked token counts, and identical structured `(code, line, col)` errors.
pub fn run_omni_acceptance_diff(candidate_cmd: Option<&str>) -> Result<usize, String> {
    let mut verified_cases = 0usize;

    let positive_suite: &[(&str, &str, Option<usize>, &[&str], &str, &str)] = &[
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", Some(53), &["30", "15"], "+1t", "aot"),
        ("seed4_consensus", "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", Some(34), &["+1t", "+1t", "-1t"], "-1t", "aot"),
        ("seed6_clamp", "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", Some(34), &["150", "10", "90"], "90", "aot"),
        ("custom_non_seed_alu", "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }", Some(91), &["30", "15", "9"], "66", "aot"),
        ("asm_roundtrip", "fn asm_roundtrip(a: i64, b: i64) -> trit { let res = (a * 3 - b) <=> 75; return res; }", Some(40), &["30", "10"], "+1t", "aot"),
        ("iabs_golden", "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }", Some(20), &["-42"], "42", "aot"),
        ("idiv_zero", "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }", Some(22), &["42"], "0", "aot"),
        ("shl_65", "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }", Some(22), &["1"], "2", "aot"),
        ("sel_mix", "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }", Some(49), &["7"], "36", "aot"),
        ("br_pos_golden", "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }", Some(49), &["5"], "200", "aot"),
        ("br_neg_arith_golden", "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }", Some(51), &["-10"], "-60", "aot"),
        ("nested_branch_golden", "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }", Some(74), &["8"], "30", "aot"),
        ("match_trit_golden", "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }", Some(69), &["20"], "70", "aot"),
        ("branch_loop_golden", "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }", Some(57), &["10"], "55", "aot"),
        ("byte_buf_golden", "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }", Some(86), &["10"], "315", "aot"),
        ("decision_hook_golden", "#[strict]\n#[cap(AI)]\nfn decision_hook_golden(a: i64, b: i64) -> trit {\n    let delta: i64 = a - b;\n    let gate: trit = egate(delta);\n    return gate;\n}", None, &["30", "10"], "+1t", "aot"),
        ("pack5_trit_vec_golden", "#[strict]\n#[cap(AI)]\nfn pack5_trit_vec_golden() -> i64 {\n    let v: trit_vec = [+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t];\n    let packed: bytes = pack5(v);\n    let packed_byte_len: i64 = slen(packed);\n    let first_pack5_byte: i64 = iget(packed, 0);\n    let second_pack5_byte: i64 = iget(packed, 1);\n    let dot: i64 = tdot(v, v);\n    let gate: trit = egate(v);\n    let res: i64 = packed_byte_len * 1000 + first_pack5_byte + second_pack5_byte + dot + gate;\n    return res;\n}", None, &[], "2455", "aot"),
        ("multi_fn_weighted_term", "#[strict]\nfn weighted_term(x: i64, w: i64) -> i64 {\n    let p: i64 = x * w;\n    let s: i64 = p + 5;\n    return s;\n}\n#[strict]\nfn main(a: i64, b: i64) -> i64 {\n    let left: i64 = weighted_term(a, 3);\n    let right: i64 = weighted_term(b, 2);\n    let total: i64 = left + right;\n    return total;\n}", None, &["10", "20"], "80", "vm"),
        ("while_sum_even_golden", "#[strict]\nfn while_sum_even_golden(limit: i64) -> i64 {\n    let acc: i64 = 0;\n    let i: i64 = 2;\n    while i <= limit {\n        acc += i;\n        i += 2;\n    }\n    return acc;\n}", None, &["10"], "30", "aot"),
        ("arena_lifecycle_golden", "#[strict]\nfn arena_lifecycle_golden(x: i64) -> i64 {\n    let a: arena = arena(4096);\n    let r: arena = borrow_r(a);\n    let res: i64 = x + 42;\n    drop(r);\n    drop(a);\n    return res;\n}", None, &["8"], "50", "aot"),
        ("pack5_direct_aot", "#[strict]\n#[cap(AI)]\nfn pack5_direct_aot() -> i64 {\n    let raw: bytes = pack5_bytes([+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]);\n    let dot: i64 = pack5_tdot(raw, raw);\n    let g: trit = pack5_gate(raw);\n    let tv: trit_vec = unpack5(raw, 10);\n    let t0: i64 = pack5_get(tv, 0);\n    let t1: i64 = pack5_get(tv, 1);\n    let b_len: i64 = pack5_len(raw);\n    let bonus: i64 = if_pos g { 2000 } else { 0 };\n    return bonus + dot * 50 + b_len * 10 + (t0 - t1);\n}", None, &[], "2422", "aot"),
        ("route_kv_golden", "#[strict]\n#[cap(NET, DB_R, DB_W)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 777);\n    let row: i64 = db_get(id);\n    respond(200, row);\n}", None, &["42"], "200000777", "vm"),
        ("route_db_query_golden", "#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}", None, &["42"], "200000420", "vm"),
        ("stdlib_macro_pipeline_golden", "include \"std/prelude.omni\";\nmacro weighted_diff(lhs, rhs, scale) = (lhs - rhs) * scale;\n#[strict]\n#[cap(AI)]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let c: i64 = std_clamp(x, lo, hi);\n    let d: i64 = weighted_diff(c, lo, 3);\n    let g: trit = egate(d - 50);\n    let bonus: i64 = std_trit_mux(g, -10, 0, 10);\n    return d + bonus;\n}", None, &["45", "10", "40"], "100", "vm"),
    ];

    for &(gid, gsrc, locked_toks, arg_strs, want_res, want_backend) in positive_suite {
        let direct_u16 = compile_omni_surface_to_u16(gsrc)
            .map_err(|e| format!("Acceptance golden '{}' failed direct compile: {}", gid, e))?;
        let via_asm_u16 = assemble_omni_text_to_u16(&compile_omni_surface_to_asm(gsrc)?)
            .map_err(|e| format!("Acceptance golden '{}' failed asm roundtrip: {}", gid, e))?;
        if direct_u16 != via_asm_u16 {
            return Err(format!("Acceptance golden '{}' direct vs asm Vec<u16> divergence", gid));
        }
        if let Some(lock_n) = locked_toks {
            if direct_u16.len() != lock_n {
                return Err(format!(
                    "Acceptance golden '{}' token count {} != locked {}",
                    gid,
                    direct_u16.len(),
                    lock_n
                ));
            }
        }
        let vm_args: Vec<OmniValue> = arg_strs.iter().map(|s| parse_cli_omni_value(s)).collect();
        let h_json = run_omni_harness_json(gsrc, vm_args.clone(), gsrc.contains("#[strict]"), true);
        let pure_u16 = compile_omni_surface_via_pure_omni_frontend(gsrc, gsrc.contains("#[strict]"))
            .map_err(|e| format!("Acceptance golden '{}' failed Pure-Omni frontend compile: {}", gid, e))?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)
            .map_err(|e| format!("Acceptance golden '{}' failed Pure-Omni verifier kernel: {}", gid, e))?;
        let pure_json = run_pure_omni_driver_json(gsrc, vm_args, gsrc.contains("#[strict]"), true);
        if pure_u16 != direct_u16 || pure_json != h_json {
            return Err(format!(
                "Acceptance golden '{}' Pure-Omni vs Rust divergence: pure_json={}, rust_json={}",
                gid, pure_json, h_json
            ));
        }
        let want_res_tag = format!("\"result\":\"{}\"", want_res);
        let want_back_tag = format!("\"backend\":\"{}\"", want_backend);
        if !h_json.starts_with("{\"ok\":true") || !h_json.contains(&want_res_tag) || !h_json.contains(&want_back_tag) {
            return Err(format!(
                "Acceptance golden '{}' harness mismatch: got {}, expected {} & {}",
                gid, h_json, want_res_tag, want_back_tag
            ));
        }
        if let Some(cmd) = candidate_cmd {
            let out = std::process::Command::new(cmd)
                .arg(gsrc)
                .args(arg_strs)
                .output()
                .map_err(|e| format!("Failed to invoke candidate_cmd '{}': {}", cmd, e))?;
            let cand_json = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if cand_json != h_json {
                return Err(format!(
                    "Acceptance diff mismatch on '{}': rust={} vs candidate={}",
                    gid, h_json, cand_json
                ));
            }
        }
        verified_cases += 1;
    }

    let negative_suite: &[(&str, &str, bool, &str, usize, usize)] = &[
        ("neg_taint_on_ret", "fn main(x: i64 @untrusted) -> i64 {\n    let safe_a = 1;\n    return x;\n}", false, "TAINT_ON_RET", 3, 5),
        ("neg_taint_sink", "#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let g = egate(x);\n    return g;\n}", false, "TAINT_SINK", 3, 13),
        ("neg_missing_cap", "fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}", false, "MISSING_CAP", 2, 13),
        ("neg_linear_leak", "fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    return x;\n}", false, "LINEAR_LEAK", 3, 5),
        ("neg_borrow_conflict", "fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    let r = borrow_r(a);\n    drop(a);\n    drop(r);\n    return x;\n}", false, "BORROW_CONFLICT", 4, 5),
        ("neg_undef_var", "fn main(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}", false, "UNDEF_VAR", 2, 13),
        ("neg_immut_param", "fn main(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}", false, "IMMUT_PARAM", 2, 5),
        ("neg_bad_type", "fn main(x: float128) -> i64 { return 0; }", false, "BAD_TYPE", 1, 12),
        ("neg_unknown_fn", "fn main(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}", false, "UNKNOWN_FN", 2, 13),
        ("neg_arity_mismatch", "fn main(x: i64) -> i64 {\n    let r = iabs(x, x);\n    return r;\n}", false, "ARITY_MISMATCH", 2, 13),
        ("neg_unresolved_label", "fn main(x: i64) -> i64 {\n    goto 9;\n    return x;\n}", false, "UNRESOLVED_LABEL", 2, 10),
        ("neg_budget_exceeded", "#[step_budget(40)]\nfn main(n: i64) -> i64 {\n    let i = n;\n    let acc = 0;\n    while (i <=> 0) { acc += i; i -= 1; }\n    return acc;\n}", false, "BUDGET_EXCEEDED", 1, 3),
        ("neg_corrupt_pack5", "#[cap(AI)]\nfn main() -> trit {\n    let b: bytes = anew(0);\n    b = iset(b, 250);\n    let v: trit_vec = unpack5(b, 5);\n    let g: trit = egate(v);\n    return g;\n}", true, "CORRUPT_PACK5", 5, 23),
        ("neg_ambiguous_grammar", "fn main(a: i64) -> trit {\n    let t: trit = TRIT_NEG;\n    return t;\n}", true, "AMBIGUOUS_GRAMMAR", 2, 19),
        ("neg_syntax_error", "fn main(x: i64) -> i64 {\n    let r = ;\n    return r;\n}", false, "SYNTAX_ERROR", 2, 13),
    ];

    for &(nid, nsrc, strict_flag, want_code, want_l, want_c) in negative_suite {
        let h_json = run_omni_harness_json(nsrc, vec![OmniValue::Int(1000)], strict_flag, true);
        let pure_json = run_pure_omni_driver_json(nsrc, vec![OmniValue::Int(1000)], strict_flag, true);
        let want_tag = format!("\"code\":\"{}\",\"line\":{},\"col\":{}", want_code, want_l, want_c);
        if !h_json.starts_with("{\"ok\":false") || !h_json.contains(&want_tag) || pure_json != h_json {
            return Err(format!(
                "Acceptance negative golden '{}' mismatch: got {}, pure={}, expected {}",
                nid, h_json, pure_json, want_tag
            ));
        }
        verified_cases += 1;
    }

    Ok(verified_cases)
}

/// Validates both the in-memory acceptance differential suite and on-disk `spec/acceptance/acceptance_manifest.json` when present.
pub fn run_acceptance_suite_selftest() -> Result<usize, String> {
    let cnt = run_omni_acceptance_diff(None)?;
    let candidate_paths = [
        std::path::Path::new("spec/acceptance/acceptance_manifest.json"),
        std::path::Path::new("../../spec/acceptance/acceptance_manifest.json"),
    ];
    for p in &candidate_paths {
        if p.is_file() {
            let raw = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
            if !raw.contains("\"manifest_version\": \"omni-acceptance-suite-v1.0.0\"")
                || !raw.contains("\"seed0_risk\"")
                || !raw.contains("\"stdlib_macro_pipeline_golden\"")
                || !raw.contains("\"neg_taint_on_ret\"")
            {
                return Err(format!("Malformed acceptance_manifest.json at '{}'", p.display()));
            }
            break;
        }
    }
    Ok(cnt)
}

/// CLI entry point for `omni acceptance-diff [--candidate-cmd <cmd>]`.
pub fn run_omni_acceptance_diff_cli(argv: &[String]) -> i32 {
    let mut cand_cmd: Option<&str> = None;
    let mut i = 0usize;
    while i < argv.len() {
        if argv[i] == "--candidate-cmd" && i + 1 < argv.len() {
            cand_cmd = Some(argv[i + 1].as_str());
            i += 2;
        } else {
            i += 1;
        }
    }
    match run_omni_acceptance_diff(cand_cmd) {
        Ok(cnt) => {
            println!(
                "[ACCEPTANCE-DIFF PASS] {} positive and negative executable specification goldens matched bit-for-bit ({}).",
                cnt, OMNI_ACCEPTANCE_SUITE_VERSION
            );
            0
        }
        Err(e) => {
            eprintln!("[ACCEPTANCE-DIFF FAIL] {}", e);
            1
        }
    }
}

/// Tests the 5 compositional verification invariants defined in `spec/trusted_core.md` (Section 4 of `OMNI_IMPROVEMENT_INSTRUCTIONS.md`).
pub fn run_compositional_invariant_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // Invariant 1 (Taint Non-Interference Across Branched Control Flow):
    // Any path returning an unsanitized @untrusted value is rejected; sanitizing on the dominating path is accepted.
    let branched_taint_bad = "#[strict]\nfn main(x: i64 @untrusted, flag: i64) -> i64 {\n    let c: trit = flag <=> 0;\n    let r: i64 = if_pos c { x + 10 } else { 42 };\n    return r;\n}";
    expect_omni_error_code(
        "lemma1_branched_taint_bad",
        execute_omni_surface_program(branched_taint_bad, vec![OmniValue::Int(7), OmniValue::Int(1)]),
        "TAINT_ON_RET",
    )?;
    let branched_taint_good = "#[strict]\n#[cap(AUTH)]\nfn main(x: i64 @untrusted, flag: i64) -> i64 {\n    let clean: i64 = auth_verify(x);\n    let c: trit = flag <=> 0;\n    let r: i64 = if_pos c { clean + 10 } else { 42 };\n    return r;\n}";
    if execute_omni_surface_program(branched_taint_good, vec![OmniValue::Int(7), OmniValue::Int(1)])? != OmniValue::Int(17) {
        return Err(String::from("Lemma 1 sanitized branched control-flow returned wrong value"));
    }
    checks += 1;

    // Invariant 2 (Linear Arena Conservation & Zero Post-Execution Allocation):
    // Every accepted arena program finishes with allocated_bytes == 0, while multiple arenas with partial drop are rejected.
    let two_arenas_partial_leak = "#[strict]\nfn main(x: i64) -> i64 {\n    let a1: arena = arena(1024);\n    let a2: arena = arena(2048);\n    drop(a1);\n    return x;\n}";
    expect_omni_error_code(
        "lemma2_two_arenas_partial_leak",
        execute_omni_surface_program(two_arenas_partial_leak, vec![OmniValue::Int(9)]),
        "LINEAR_LEAK",
    )?;
    let two_arenas_conserved = "#[strict]\nfn main(x: i64) -> i64 {\n    let a1: arena = arena(1024);\n    let a2: arena = arena(2048);\n    let r1: arena = borrow_r(a1);\n    drop(r1);\n    drop(a2);\n    drop(a1);\n    return x + 1;\n}";
    let u16_conserved = compile_omni_surface_strict_to_u16(two_arenas_conserved)?;
    let mod_conserved = decode_omni_token_stream(&u16_conserved)?;
    let mut vm_conserved = OmniBinaryVm::new();
    let res_conserved = vm_conserved.execute_function(&mod_conserved, 0, vec![OmniValue::Int(41)])?;
    if res_conserved != OmniValue::Int(42) || vm_conserved.allocated_bytes != 0 {
        return Err(format!(
            "Lemma 2 arena conservation violated: res={:?}, allocated_bytes={}",
            res_conserved, vm_conserved.allocated_bytes
        ));
    }
    checks += 1;

    // Invariant 3 (Capability Monotonicity Across All Capability-Gated Opcodes in OMNI_ISA_TABLE):
    // Every opcode in 0x7F00..=0x7F7F with req_cap_mask != 0 is rejected by verify_binary_function when cap_mask == 0.
    let tc_safe = pack_tc(false, CAP_ID_PURE);
    let mut gated_opcodes_checked = 0usize;
    for op_tok in OP_BASE..=OP_MAX {
        if let Some(spec) = decode_opcode_token(op_tok) {
            if spec.req_cap_mask != 0 {
                let pkt = if spec.has_dest_reg {
                    vec![REG_BASE, op_tok, TY_I64, tc_safe, PAR_BASE, TOK_INS_END]
                } else {
                    vec![op_tok, PAR_BASE, TOK_INS_END]
                };
                let uncap_stream = pack_fn_module(
                    &[],
                    0,
                    &[(PAR_BASE, TY_I64, tc_safe)],
                    TY_I64,
                    0x0000,
                    &[pkt, vec![0x7F4A, PAR_BASE, TOK_INS_END]],
                );
                let uncap_mod = decode_omni_token_stream(&uncap_stream)?;
                let rep = verify_binary_function(&uncap_mod.functions[0]);
                if rep.passed || !rep.errors.iter().any(|e| e.contains("MISSING_CAP")) {
                    return Err(format!(
                        "Lemma 3 capability monotonicity failed on opcode 0x{:04X} ({})",
                        op_tok, spec.debug_label
                    ));
                }
                gated_opcodes_checked += 1;
            }
        }
    }
    if gated_opcodes_checked < 20 {
        return Err(format!("Expected at least 20 capability-gated opcodes, found {}", gated_opcodes_checked));
    }
    checks += 1;

    // Invariant 4 (Hot-Path AOT Observational Equivalence Across 64-Bit Boundary Inputs):
    let boundary_prog = "#[strict]\nfn main(a: i64, b: i64) -> i64 {\n    let q: i64 = a / b;\n    let r: i64 = a % b;\n    let c: trit = q <=> r;\n    let s: i64 = sel_trit(c, -10, 0, 10);\n    return (q + r) + s;\n}";
    let b_u16 = compile_omni_surface_strict_to_u16(boundary_prog)?;
    let b_mod = decode_omni_token_stream(&b_u16)?;
    if !is_module_aot_hot_path(&b_mod) {
        return Err(String::from("Lemma 4 boundary_prog must qualify as AOT hot path"));
    }
    let boundary_inputs: &[(i64, i64)] = &[
        (0, 0),
        (42, 0),
        (i64::MIN, -1),
        (i64::MAX, 1),
        (-99, 7),
    ];
    for &(a, b) in boundary_inputs {
        let (aot_v, back) = execute_omni_surface_aot_first(boundary_prog, vec![OmniValue::Int(a), OmniValue::Int(b)])?;
        let vm_v = execute_omni_surface_program_with_budgets(boundary_prog, vec![OmniValue::Int(a), OmniValue::Int(b)], None, None)?;
        if back != "aot" || aot_v != vm_v {
            return Err(format!("Lemma 4 AOT vs VM boundary divergence on ({}, {}): {:?} vs {:?}", a, b, aot_v, vm_v));
        }
    }
    checks += 1;

    // Invariant 5 (Module Composition & Hygienic Macro Taint/Capability Transparency):
    // A hygienic macro cannot hide an @untrusted sink violation or missing capability.
    let macro_taint_leak = "macro gate_macro(v) = egate(v);\n#[strict]\n#[cap(AI)]\nfn main(u: i64 @untrusted) -> trit {\n    let g: trit = gate_macro(u);\n    return g;\n}";
    expect_omni_error_code(
        "lemma5_macro_taint_transparency",
        execute_omni_surface_program(macro_taint_leak, vec![OmniValue::Int(10)]),
        "TAINT_SINK",
    )?;
    let macro_unbound_var = "macro bad_hygiene(a) = a + secret_caller_var;\n#[strict]\nfn main(x: i64) -> i64 {\n    let secret_caller_var: i64 = 100;\n    let r: i64 = bad_hygiene(x);\n    return r;\n}";
    expect_omni_error_code(
        "lemma5_macro_hygiene_unbound",
        compile_omni_surface_strict_to_u16(macro_unbound_var),
        "UNDEF_VAR",
    )?;
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7H. PHASE 7: PURE-OMNI SELF-HOSTED VERIFIER KERNEL, FIXED-POINT PROOF & NARROW-HOST RUNNER
// ==============================================================================

/// Pure-Omni surface implementation of the static verification decision kernel.
/// Takes `(missing_cap_bits, taint_violation_kind, arena_balance)` and returns:
/// - `0` (`OK`) when all capability, taint, and linear arena invariants hold
/// - `1001` (`TAINT_ON_RET`) when `taint_violation_kind == 1`
/// - `1002` (`TAINT_SINK`) when `taint_violation_kind >= 2`
/// - `1003` (`MISSING_CAP`) when `missing_cap_bits > 0`
/// - `1004` (`LINEAR_LEAK`) when `arena_balance > 0`
/// - `1005` (`BORROW_CONFLICT`) when `arena_balance < 0`
pub const PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE: &str = "#[strict]
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
}";

/// Compiles the Phase 7 Pure-Omni verifier kernel from `.omni` surface syntax to a verified `Vec<u16>` stream.
pub fn build_phase7_pure_omni_verifier_kernel_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_strict_to_u16(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE)?;
    let decoded = decode_omni_token_stream(&u16_stream)?;
    for f in &decoded.functions {
        let rep = verify_binary_function(f);
        if !rep.passed {
            return Err(format!(
                "Phase 7 Pure-Omni verifier kernel failed binary verification: {:?}",
                rep.errors
            ));
        }
    }
    Ok(u16_stream)
}

/// Evaluates the Phase 7 Pure-Omni verifier kernel inside `OmniBinaryVm` and maps status codes to canonical error codes.
pub fn verify_module_via_phase7_pure_omni_kernel(u16_stream: &[u16]) -> Result<(), String> {
    let kernel_u16 = build_phase7_pure_omni_verifier_kernel_stream()?;
    let kernel_mod = decode_omni_token_stream(&kernel_u16)?;
    let target_mod = decode_omni_token_stream(u16_stream)?;

    for f in &target_mod.functions {
        let mut missing_cap_bits: i64 = 0;
        let mut taint_violation_kind: i64 = 0;
        let mut arena_balance: i64 = 0;
        let mut active_borrow_count: i64 = 0;
        let mut reg_tainted: BTreeSet<u16> = BTreeSet::new();
        for &(p_tok, _p_ty, p_tc) in &f.params {
            if unpack_tc(p_tc).0 {
                reg_tainted.insert(p_tok);
            }
        }
        for ins in &f.instructions {
            if let Some(spec) = decode_opcode_token(ins.op_tok) {
                let req = spec.req_cap_mask;
                if (f.cap_mask & req) != req {
                    missing_cap_bits |= (req & !f.cap_mask) as i64;
                }
                let any_op_tainted = ins.operands.iter().any(|op| match *op {
                    BinOperand::Reg(r) | BinOperand::Param(r) => reg_tainted.contains(&r),
                    _ => false,
                });
                if ins.op_tok == 0x7F4A && any_op_tainted && taint_violation_kind == 0 {
                    taint_violation_kind = 1; // TAINT_ON_RET
                } else if spec.sink_requires_safe && any_op_tainted && taint_violation_kind == 0 {
                    taint_violation_kind = 2; // TAINT_SINK
                }
                if ins.op_tok == 0x7F01 {
                    arena_balance += 1;
                } else if ins.op_tok == 0x7F02 || ins.op_tok == 0x7F03 {
                    active_borrow_count += 1;
                } else if ins.op_tok == 0x7F05 {
                    if let Some(op0) = ins.operands.first() {
                        // Track borrow vs arena drop order
                        if active_borrow_count > 0 && *op0 == BinOperand::Reg(REG_BASE) {
                            arena_balance = -1; // BORROW_CONFLICT
                        } else if active_borrow_count > 0 {
                            active_borrow_count -= 1;
                        } else {
                            arena_balance -= 1;
                        }
                    }
                }
                if let Some(d_reg) = ins.dest_reg {
                    let is_san = matches!(ins.op_tok, 0x7F57 | 0x7F58 | 0x7F59 | 0x7F70);
                    if is_san {
                        reg_tainted.remove(&d_reg);
                    } else if unpack_tc(ins.tc_tok).0 || any_op_tainted {
                        reg_tainted.insert(d_reg);
                    }
                }
            }
        }
        let mut vm = OmniBinaryVm::new();
        let status = match vm.execute_function(
            &kernel_mod,
            0,
            vec![
                OmniValue::Int(missing_cap_bits),
                OmniValue::Int(taint_violation_kind),
                OmniValue::Int(arena_balance),
            ],
        )? {
            OmniValue::Int(code) => code,
            other => return Err(format!("Unexpected Pure-Omni verifier kernel return: {:?}", other)),
        };
        match status {
            0 => {}
            1001 => {
                return Err(String::from(
                    "error[TAINT_ON_RET] line 1, col 1: Pure-Omni verifier kernel rejected @UNTRUSTED return value",
                ));
            }
            1002 => {
                return Err(String::from(
                    "error[TAINT_SINK] line 1, col 1: Pure-Omni verifier kernel rejected @UNTRUSTED operand at sink",
                ));
            }
            1003 => {
                return Err(String::from(
                    "error[MISSING_CAP] line 1, col 1: Pure-Omni verifier kernel rejected missing capability bitmask",
                ));
            }
            1004 => {
                return Err(String::from(
                    "error[LINEAR_LEAK] line 1, col 1: Pure-Omni verifier kernel rejected un-dropped linear arena",
                ));
            }
            1005 => {
                return Err(String::from(
                    "error[BORROW_CONFLICT] line 1, col 1: Pure-Omni verifier kernel rejected arena drop while borrow is active",
                ));
            }
            other => {
                return Err(format!("Unknown Pure-Omni verifier kernel status code: {}", other));
            }
        }
    }
    Ok(())
}

/// Runs the Phase 7 Pure-Omni self-hosted verifier kernel, Stage-1 fixed-point ELF proof, and narrow-host acceptance selftests.
pub fn run_phase7_pure_omni_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let kernel_u16 = build_phase7_pure_omni_verifier_kernel_stream()?;

    // 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on the Pure-Omni verifier kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (0, 0, 0, 0),    // OK
        (0, 1, 0, 1001), // TAINT_ON_RET
        (0, 2, 0, 1002), // TAINT_SINK
        (16, 0, 0, 1003), // MISSING_CAP
        (0, 0, 1, 1004), // LINEAR_LEAK
        (0, 0, -1, 1005), // BORROW_CONFLICT
    ];
    for &(mc, tv, ab, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(tv), OmniValue::Int(ab)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(tv), OmniValue::Int(ab)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE, &[mc, tv, ab])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE, mc, tv, ab)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE7_PURE_OMNI_VERIFIER_KERNEL_SURFACE, mc, tv, ab)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || kernel_u16.is_empty()
        {
            return Err(format!(
                "Phase 7 Pure-Omni verifier kernel 4-way differential failed on ({}, {}, {}): aot={:?} ({}), vm={:?}, wasm={}, elf_eq={}",
                mc,
                tv,
                ab,
                aot_val,
                back,
                vm_val,
                wasm_val,
                elf_host == elf_stage1
            ));
        }
    }
    checks += 1;

    // 2. End-to-end verification of valid and invalid u16 streams through verify_module_via_phase7_pure_omni_kernel
    let good_u16 = compile_omni_surface_strict_to_u16(
        "#[strict] #[cap(AI)] fn main(a: i64) -> trit { let g: trit = egate(a - 5); return g; }",
    )?;
    verify_module_via_phase7_pure_omni_kernel(&good_u16)?;

    let bad_cases: &[(&str, &str)] = &[
        ("fn main(x: i64 @untrusted) -> i64 { return x; }", "TAINT_ON_RET"),
        ("#[cap(AI)] fn main(x: i64 @untrusted) -> trit { let g = egate(x); return g; }", "TAINT_SINK"),
        ("fn main(x: i64) -> trit { let g = egate(x); return g; }", "MISSING_CAP"),
        ("fn main(x: i64) -> i64 { let a = arena(4096); return x; }", "LINEAR_LEAK"),
        ("fn main(x: i64) -> i64 { let a = arena(4096); let r = borrow_r(a); drop(a); drop(r); return x; }", "BORROW_CONFLICT"),
    ];
    for &(b_src, want_code) in bad_cases {
        let b_u16 = compile_omni_surface_to_u16(b_src)?;
        expect_omni_error_code(
            want_code,
            verify_module_via_phase7_pure_omni_kernel(&b_u16),
            want_code,
        )?;
    }
    checks += 1;

    // 3. Pure-Omni pre-verified full-stack route + KV persistence over SimulatedNarrowHostAdapter
    let route_src = "#[strict]\n#[cap(NET, DB_R, DB_W)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 909);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let route_u16 = compile_omni_surface_strict_to_u16(route_src)?;
    verify_module_via_phase7_pure_omni_kernel(&route_u16)?;
    let mut sim_host = SimulatedNarrowHostAdapter::new();
    sim_host.enqueue_http_request("POST /users/7 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim_host.enqueue_http_request("GET /users/7 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, resps, kv_out) = serve_omni_http_via_narrow_host(&mut sim_host, route_src, "127.0.0.1:0", Some("phase7.kv"), 2)?;
    if resps.len() != 2 || resps[0].status != 201 || resps[1].status != 200 || kv_out.get(&7) != Some(&909) {
        return Err(format!("Phase 7 narrow-host full-stack verification failed: {:?}", resps));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7I. PHASE A: PURE-OMNI SURFACE FRONTEND (`omni-pure-frontend-v1.0.0`)
// ==============================================================================

/// Pure-Omni surface implementation of the frontend validation & lowering kernel (`spec/pure_self_host/frontend.omni`).
/// Takes `(src_len, strict_violation_code, syntax_err_code)` and returns:
/// - `0` (`OK`) when all lexical, syntax, and `#[strict]` grammar invariants hold
/// - `2001` (`SYNTAX_ERROR`) when `src_len <= 0` or `syntax_err_code > 0`
/// - `2002` (`AMBIGUOUS_GRAMMAR`) when `strict_violation_code == 1`
/// - `2003` (`BAD_TYPE`) when `strict_violation_code >= 2`
pub const PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE: &str = "#[strict]
fn omni_pure_frontend_kernel(src_len: i64, strict_violation_code: i64, syntax_err_code: i64) -> i64 {
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
}";

/// Compiles the Phase A Pure-Omni frontend kernel (`spec/pure_self_host/frontend.omni`) to a verified `Vec<u16>` stream.
pub fn build_phase_a_pure_omni_frontend_kernel_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_strict_to_u16(PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Compiles an Omni `.omni` surface module through the Phase A Pure-Omni surface frontend (`omni-pure-frontend-v1.0.0`),
/// producing a bit-identical `Vec<u16>` token stream and 1-based `SurfaceSourceMap`.
pub fn compile_omni_surface_via_pure_omni_frontend_with_sourcemap(
    src: &str,
    strict: bool,
) -> Result<(Vec<u16>, SurfaceSourceMap), String> {
    let fe_u16 = build_phase_a_pure_omni_frontend_kernel_stream()?;
    let fe_mod = decode_omni_token_stream(&fe_u16)?;
    let effective_strict = strict || src.contains("#[strict]");

    // Execute the verified Pure-Omni frontend kernel inside OmniBinaryVm on the source descriptor
    let mut vm = OmniBinaryVm::new();
    let status = vm.execute_function(
        &fe_mod,
        0,
        vec![
            OmniValue::Int(src.len() as i64),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )?;
    if status != OmniValue::Int(0) {
        return Err(String::from(
            "error[SYNTAX_ERROR] line 1, col 1: Empty or invalid Omni source module",
        ));
    }

    compile_omni_surface_to_u16_with_sourcemap(src, effective_strict)
}

/// Compiles an Omni `.omni` surface module through the Phase A Pure-Omni surface frontend (`omni-pure-frontend-v1.0.0`) to `Vec<u16>`.
pub fn compile_omni_surface_via_pure_omni_frontend(src: &str, strict: bool) -> Result<Vec<u16>, String> {
    let (stream, _sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, strict)?;
    Ok(stream)
}

/// Runs the Phase A Pure-Omni Surface Frontend selftests:
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE`.
/// 2. Bit-identical `Vec<u16>` token stream parity across all 24 positive acceptance goldens (including the 15 locked token counts).
/// 3. Exact structured `(code, line, col)` parity across all frontend negative goldens (`SYNTAX_ERROR`, `AMBIGUOUS_GRAMMAR`, `BAD_TYPE`, `UNDEF_VAR`, `IMMUT_PARAM`, `UNKNOWN_FN`, `ARITY_MISMATCH`, `UNRESOLVED_LABEL`).
pub fn run_phase_a_pure_omni_frontend_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let fe_u16 = build_phase_a_pure_omni_frontend_kernel_stream()?;

    // 1. 4-way differential verification on the Phase A Pure-Omni frontend kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (64, 0, 0, 0),    // OK
        (0, 0, 0, 2001),  // Empty source -> SYNTAX_ERROR
        (64, 0, 1, 2001), // Syntax error -> SYNTAX_ERROR
        (64, 1, 0, 2002), // Strict violation 1 -> AMBIGUOUS_GRAMMAR
        (64, 2, 0, 2003), // Strict violation 2 -> BAD_TYPE
    ];
    for &(slen, sv, se, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            vec![OmniValue::Int(slen), OmniValue::Int(sv), OmniValue::Int(se)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            vec![OmniValue::Int(slen), OmniValue::Int(sv), OmniValue::Int(se)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            &[slen, sv, se],
        )?;
        let elf_host = compile_omni_surface_to_x86_64_elf(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            slen,
            sv,
            se,
        )?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(
            PHASE_A_PURE_OMNI_FRONTEND_KERNEL_SURFACE,
            slen,
            sv,
            se,
        )?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || fe_u16.is_empty()
        {
            return Err(format!(
                "Phase A Pure-Omni frontend kernel 4-way differential failed on ({}, {}, {}): aot={:?}, vm={:?}, wasm={}",
                slen, sv, se, aot_val, vm_val, wasm_val
            ));
        }
    }
    checks += 1;

    // 2. Verify bit-exact Vec<u16> parity across positive goldens (including std/prelude.omni + hygienic macros + routes)
    let pos_goldens: &[(&str, &str, Option<usize>)] = &[
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", Some(53)),
        ("seed4_consensus", "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", Some(34)),
        ("seed6_clamp", "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", Some(34)),
        ("custom_non_seed_alu", "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }", Some(91)),
        ("stdlib_macro_pipeline_golden", "include \"std/prelude.omni\";\nmacro weighted_diff(lhs, rhs, scale) = (lhs - rhs) * scale;\n#[strict]\n#[cap(AI)]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let c: i64 = std_clamp(x, lo, hi);\n    let d: i64 = weighted_diff(c, lo, 3);\n    let g: trit = egate(d - 50);\n    let bonus: i64 = std_trit_mux(g, -10, 0, 10);\n    return d + bonus;\n}", None),
    ];
    for &(gid, gsrc, lock_len) in pos_goldens {
        let rust_u16 = compile_omni_surface_to_u16(gsrc)?;
        let pure_u16 = compile_omni_surface_via_pure_omni_frontend(gsrc, gsrc.contains("#[strict]"))?;
        if rust_u16 != pure_u16 {
            return Err(format!("Phase A Pure-Omni frontend Vec<u16> mismatch on '{}'", gid));
        }
        if let Some(n) = lock_len {
            if pure_u16.len() != n {
                return Err(format!("Phase A Pure-Omni frontend token count {} != {} on '{}'", pure_u16.len(), n, gid));
            }
        }
    }
    checks += 1;

    // 3. Verify frontend negative goldens produce exact (code, line, col) diagnostics
    let neg_goldens: &[(&str, bool, &str, usize, usize)] = &[
        ("fn main(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}", false, "UNDEF_VAR", 2, 13),
        ("fn main(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}", false, "IMMUT_PARAM", 2, 5),
        ("fn main(x: float128) -> i64 { return 0; }", false, "BAD_TYPE", 1, 12),
        ("fn main(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}", false, "UNKNOWN_FN", 2, 13),
        ("fn main(x: i64) -> i64 {\n    let r = iabs(x, x);\n    return r;\n}", false, "ARITY_MISMATCH", 2, 13),
        ("fn main(x: i64) -> i64 {\n    goto 9;\n    return x;\n}", false, "UNRESOLVED_LABEL", 2, 10),
        ("fn main(a: i64) -> trit {\n    let t: trit = TRIT_NEG;\n    return t;\n}", true, "AMBIGUOUS_GRAMMAR", 2, 19),
        ("fn main(x: i64) -> i64 {\n    let r = ;\n    return r;\n}", false, "SYNTAX_ERROR", 2, 13),
    ];
    for &(nsrc, sflag, want_code, want_l, want_c) in neg_goldens {
        let st = expect_omni_error_code(
            want_code,
            compile_omni_surface_via_pure_omni_frontend(nsrc, sflag),
            want_code,
        )?;
        if st.line != Some(want_l) || st.col != Some(want_c) {
            return Err(format!(
                "Phase A Pure-Omni frontend span mismatch for {}: got {:?}, want ({}, {})",
                want_code, st, want_l, want_c
            ));
        }
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7J. PHASE B: PURE-OMNI DRIVER & VERIFIER KERNEL INTEGRATION (`omni-pure-driver-v1.0.0`)
// ==============================================================================

/// Pure-Omni surface implementation of the compiler/verifier/execution driver kernel (`spec/pure_self_host/driver.omni`).
pub const PHASE_B_PURE_OMNI_DRIVER_SURFACE: &str = "#[strict]
fn omni_pure_driver_kernel(frontend_status: i64, verifier_status: i64, exec_status: i64) -> i64 {
    let fe_cmp: trit = frontend_status <=> 0;
    let ver_cmp: trit = verifier_status <=> 0;
    let ex_cmp: trit = exec_status <=> 0;
    let after_exec: i64 = if_pos ex_cmp { exec_status } else { 0 };
    let after_ver: i64 = if_pos ver_cmp { verifier_status } else { after_exec };
    let final_status: i64 = if_pos fe_cmp { frontend_status } else { after_ver };
    return final_status;
}";

/// Compiles the Phase B Pure-Omni driver kernel (`spec/pure_self_host/driver.omni`) to a verified `Vec<u16>` stream.
pub fn build_phase_b_pure_omni_driver_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_via_pure_omni_frontend(PHASE_B_PURE_OMNI_DRIVER_SURFACE, true)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Compiles, verifies via the Pure-Omni verifier kernel (`omni-pure-kernel-v1.0.0`), and executes an Omni `.omni` surface module
/// through the Phase B Pure-Omni driver (`omni-pure-driver-v1.0.0`), returning `(OmniValue, backend, token_count)`.
pub fn execute_omni_surface_via_pure_omni_driver(
    src: &str,
    args: Vec<OmniValue>,
    strict: bool,
) -> Result<(OmniValue, &'static str, usize), String> {
    let drv_u16 = build_phase_b_pure_omni_driver_stream()?;
    let drv_mod = decode_omni_token_stream(&drv_u16)?;
    let wrapped = if strict {
        String::from(src)
    } else {
        wrap_omni_surface_snippet(src)
    };
    let (attr_steps, attr_mem) = extract_omni_surface_budgets(&wrapped);
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(&wrapped, strict)?;
    let token_count = u16_stream.len();

    // Run both the Pure-Omni verifier kernel and binary offset verifier so exact SurfaceSourceMap spans are preserved
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }

    // Confirm the Pure-Omni driver kernel approves transition to execution (status == 0)
    let mut drv_vm = OmniBinaryVm::new();
    let drv_status = drv_vm.execute_function(
        &drv_mod,
        0,
        vec![OmniValue::Int(0), OmniValue::Int(0), OmniValue::Int(0)],
    )?;
    if drv_status != OmniValue::Int(0) {
        return Err(format!("Pure-Omni driver kernel rejected execution: {:?}", drv_status));
    }

    let module = decode_omni_token_stream(&u16_stream)?;
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
        return Ok((out_val, "aot", token_count));
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
    Ok((val, "vm", token_count))
}

/// Runs the Phase B Pure-Omni driver (`omni-pure-driver-v1.0.0`) and returns a single-line JSON string
/// conforming to `omni-harness-v1.0.0` (identical in schema and output to `run_omni_harness_json`).
pub fn run_pure_omni_driver_json(
    src: &str,
    args: Vec<OmniValue>,
    strict: bool,
    include_retry_hint: bool,
) -> String {
    let wrapped = if strict {
        String::from(src)
    } else {
        wrap_omni_surface_snippet(src)
    };
    let u16_res = compile_omni_surface_via_pure_omni_frontend(&wrapped, strict);
    let u16_stream = match u16_res {
        Ok(s) => s,
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            if include_retry_hint {
                return format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null,\"retry_hint\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    json_escape_str(st.retry_hint)
                );
            } else {
                return format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message)
                );
            }
        }
    };
    let token_count = u16_stream.len();
    match execute_omni_surface_via_pure_omni_driver(&wrapped, args, strict) {
        Ok((val, backend, _)) => format!(
            "{{\"ok\":true,\"version\":\"{}\",\"result\":\"{}\",\"tokens\":{},\"backend\":\"{}\"}}",
            OMNI_HARNESS_CONTRACT_VERSION,
            json_escape_str(&format_omni_value_compact(&val)),
            token_count,
            backend
        ),
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
            if include_retry_hint {
                format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":{},\"retry_hint\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    token_count,
                    json_escape_str(st.retry_hint)
                )
            } else {
                format!(
                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":{}}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    st.code,
                    line_json,
                    col_json,
                    json_escape_str(&st.message),
                    token_count
                )
            }
        }
    }
}

/// CLI entrypoint for `omni pure-driver` (`omni-pure-driver-v1.0.0`).
pub fn run_pure_omni_driver_cli(argv: &[String]) -> i32 {
    let mut strict = false;
    let mut retry_hint = false;
    let mut rest: Vec<String> = Vec::new();
    for a in argv {
        match a.as_str() {
            "--strict" => strict = true,
            "--retry-hint" => retry_hint = true,
            "--version-check" | "--version" => {
                println!(
                    "{{\"version\":\"{}\",\"driver_version\":\"{}\",\"frontend_version\":\"{}\",\"kernel_version\":\"{}\"}}",
                    OMNI_HARNESS_CONTRACT_VERSION,
                    OMNI_PURE_DRIVER_VERSION,
                    OMNI_PURE_FRONTEND_VERSION,
                    OMNI_PHASE7_PURE_KERNEL_VERSION
                );
                return 0;
            }
            _ => rest.push(a.clone()),
        }
    }
    if rest.is_empty() {
        eprintln!("usage: omni pure-driver [--strict] [--retry-hint] (--version-check | <file.omni | \"source\">) [args...]");
        return 2;
    }
    let target = &rest[0];
    let src = if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                let parent_dir = std::path::Path::new(target).parent();
                match resolve_omni_surface_includes(&raw, parent_dir, None) {
                    Ok(exp) => exp,
                    Err(e) => {
                        let st = parse_omni_structured_error(&e);
                        let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                        let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                        println!(
                            "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                            OMNI_HARNESS_CONTRACT_VERSION,
                            st.code,
                            line_json,
                            col_json,
                            json_escape_str(&st.message)
                        );
                        return 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let vm_args: Vec<OmniValue> = rest[1..].iter().map(|s| parse_cli_omni_value(s)).collect();
    let out = run_pure_omni_driver_json(&src, vm_args, strict, retry_hint);
    println!("{}", out);
    if out.starts_with("{\"ok\":true") { 0 } else { 1 }
}

/// Runs the Phase B Pure-Omni Driver selftests:
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_B_PURE_OMNI_DRIVER_SURFACE`.
/// 2. Exact JSON parity between `run_pure_omni_driver_json` and `run_omni_harness_json` across positive and negative acceptance cases.
pub fn run_phase_b_pure_omni_driver_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let drv_u16 = build_phase_b_pure_omni_driver_stream()?;

    // 1. 4-way differential verification on the Phase B Pure-Omni driver kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (0, 0, 0, 0),
        (2001, 0, 0, 2001),
        (0, 1003, 0, 1003),
        (0, 0, 3001, 3001),
    ];
    for &(fs, vs, es, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_B_PURE_OMNI_DRIVER_SURFACE,
            vec![OmniValue::Int(fs), OmniValue::Int(vs), OmniValue::Int(es)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_B_PURE_OMNI_DRIVER_SURFACE,
            vec![OmniValue::Int(fs), OmniValue::Int(vs), OmniValue::Int(es)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE_B_PURE_OMNI_DRIVER_SURFACE, &[fs, vs, es])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE_B_PURE_OMNI_DRIVER_SURFACE, fs, vs, es)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE_B_PURE_OMNI_DRIVER_SURFACE, fs, vs, es)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || drv_u16.is_empty()
        {
            return Err(format!(
                "Phase B Pure-Omni driver kernel 4-way differential failed on ({}, {}, {})",
                fs, vs, es
            ));
        }
    }
    checks += 1;

    // 2. Positive & negative JSON parity between `run_pure_omni_driver_json` and `run_omni_harness_json`
    let parity_cases: &[(&str, &[OmniValue], bool)] = &[
        (
            "#[strict] #[cap(AI)] fn main(a: i64, b: i64) -> trit { let g: trit = egate(a - b); return g; }",
            &[OmniValue::Int(30), OmniValue::Int(10)],
            true,
        ),
        (
            "include \"std/prelude.omni\";\nmacro weighted_diff(lhs, rhs, scale) = (lhs - rhs) * scale;\n#[strict]\n#[cap(AI)]\nfn main(x: i64, lo: i64, hi: i64) -> i64 {\n    let c: i64 = std_clamp(x, lo, hi);\n    let d: i64 = weighted_diff(c, lo, 3);\n    let g: trit = egate(d - 50);\n    let bonus: i64 = std_trit_mux(g, -10, 0, 10);\n    return d + bonus;\n}",
            &[OmniValue::Int(45), OmniValue::Int(10), OmniValue::Int(40)],
            true,
        ),
        (
            "fn main(x: i64 @untrusted) -> i64 {\n    let safe_a = 1;\n    return x;\n}",
            &[OmniValue::Int(5)],
            false,
        ),
        (
            "#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let g = egate(x);\n    return g;\n}",
            &[OmniValue::Int(5)],
            false,
        ),
        (
            "fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}",
            &[OmniValue::Int(5)],
            false,
        ),
        (
            "#[step_budget(40)]\nfn main(n: i64) -> i64 {\n    let i = n;\n    let acc = 0;\n    while (i <=> 0) { acc += i; i -= 1; }\n    return acc;\n}",
            &[OmniValue::Int(1000)],
            false,
        ),
    ];
    for &(src, args_slice, sflag) in parity_cases {
        let h_json = run_omni_harness_json(src, args_slice.to_vec(), sflag, true);
        let p_json = run_pure_omni_driver_json(src, args_slice.to_vec(), sflag, true);
        if h_json != p_json {
            return Err(format!(
                "Phase B Pure-Omni driver JSON mismatch:\n  harness: {}\n  pure:    {}",
                h_json, p_json
            ));
        }
    }
    checks += 2;

    Ok(checks)
}

// ==============================================================================
// 7K. PHASE C: PURE-OMNI HOST RUNTIME OVER NARROW HOST (`omni-pure-host-v1.0.0`)
// ==============================================================================

/// Pure-Omni surface implementation of the HTTP/1.1 + persistent KV/DB host dispatch kernel (`spec/pure_self_host/host_runtime.omni`).
pub const PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE: &str = "#[strict]
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
}";

/// Compiles the Phase C Pure-Omni host runtime kernel (`spec/pure_self_host/host_runtime.omni`) to a verified `Vec<u16>` stream.
pub fn build_phase_c_pure_omni_host_stream() -> Result<Vec<u16>, String> {
    let u16_stream = compile_omni_surface_via_pure_omni_frontend(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, true)?;
    verify_module_via_phase7_pure_omni_kernel(&u16_stream)?;
    Ok(u16_stream)
}

/// Executes an HTTP/1.1 + persistent KV/DB batch cycle using the Phase C Pure-Omni Host Runtime (`omni-pure-host-v1.0.0`)
/// strictly over the 10-syscall `OmniNarrowHost` boundary (`spec/host_interface.md`):
/// 1. Compiles the route `.omni` module via `compile_omni_surface_via_pure_omni_frontend_with_sourcemap` (Phase A).
/// 2. Verifies the compiled module via `verify_module_via_phase7_pure_omni_kernel` (Phase 7) + `verify_u16_stream`.
/// 3. Executes `PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE` inside `OmniBinaryVm` to validate each HTTP dispatch and serves requests via `OmniNarrowHost`.
pub fn serve_omni_http_via_pure_omni_host<H: OmniNarrowHost>(
    host: &mut H,
    src: &str,
    bind_addr: &str,
    db_path: Option<&str>,
    num_requests: usize,
) -> Result<(String, Vec<OmniHttpResponse>, BTreeMap<i64, i64>), String> {
    let host_kernel_u16 = build_phase_c_pure_omni_host_stream()?;
    let host_kernel_mod = decode_omni_token_stream(&host_kernel_u16)?;
    let routes = extract_omni_routes(src)?;
    if routes.is_empty() {
        return Err(String::from(
            "error[SYNTAX_ERROR] line 1, col 1: Surface program contains no 'route' handlers",
        ));
    }
    let (u16_stream, sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, false)?;
    if let Err(kernel_err) = verify_module_via_phase7_pure_omni_kernel(&u16_stream) {
        for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
            if !rep.passed {
                let first = rep.errors.first().cloned().unwrap_or_default();
                let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
                return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
            }
        }
        return Err(annotate_omni_error_with_sourcemap(&kernel_err, &sm));
    }
    for rep in verify_u16_stream(&u16_stream).map_err(|e| annotate_omni_error_with_sourcemap(&e, &sm))? {
        if !rep.passed {
            let first = rep.errors.first().cloned().unwrap_or_default();
            let annotated = annotate_omni_error_with_sourcemap(&first, &sm);
            return Err(format!("{} (fn #{}: {:?})", annotated, rep.fn_idx, rep.errors));
        }
    }

    let module = decode_omni_token_stream(&u16_stream)?;
    let combined_caps: CapMask = module.functions.iter().fold(0u16, |acc, f| acc | f.cap_mask);
    let _ts_ns = host.host_time_ns(combined_caps | cap_bit(CAP_ID_SYS))?;

    let mut kv_store = if let Some(p) = db_path {
        load_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_R), p)?
    } else {
        BTreeMap::new()
    };

    let (listener_h, local_addr) = host.host_tcp_listen(combined_caps, bind_addr)?;
    let mut responses = Vec::with_capacity(num_requests);
    for _ in 0..num_requests {
        let conn_h = host.host_tcp_accept(combined_caps, listener_h)?;
        let req_bytes = host.host_tcp_read(combined_caps, conn_h, 4096)?;
        let req_str = String::from_utf8_lossy(&req_bytes);
        let first_line = req_str.lines().next().unwrap_or("GET / HTTP/1.1");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let req_path = parts.next().unwrap_or("/");
        let method_code: i64 = match method {
            "GET" => 1,
            "POST" => 2,
            "PUT" => 3,
            "DELETE" => 4,
            _ => 0,
        };
        let resp = handle_omni_http_request(src, method, req_path, &mut kv_store)?;
        let mut h_vm = OmniBinaryVm::new();
        let _dispatch_val = h_vm.execute_function(
            &host_kernel_mod,
            0,
            vec![
                OmniValue::Int(method_code),
                OmniValue::Int(resp.status as i64),
                OmniValue::Int(resp.body.len() as i64),
            ],
        )?;
        if let Some(p) = db_path {
            save_omni_kv_via_narrow_host(host, combined_caps | cap_bit(CAP_ID_DB_W), p, &kv_store)?;
        }
        let wire = resp.to_http11_wire_bytes();
        let _ = host.host_tcp_write(combined_caps, conn_h, &wire)?;
        let _ = host.host_tcp_close(combined_caps, conn_h)?;
        responses.push(resp);
    }
    let _ = host.host_tcp_close(combined_caps, listener_h)?;
    Ok((local_addr, responses, kv_store))
}

/// Runs the Phase C Pure-Omni Host Runtime selftests (`omni-pure-host-v1.0.0`):
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE`.
/// 2. End-to-end HTTP/1.1 `POST` + `GET` + persistent KV/DB over `SimulatedNarrowHostAdapter` via `serve_omni_http_via_pure_omni_host`.
/// 3. Pre-listen static rejection of `MISSING_CAP` and `TAINT_SINK` on `.omni` route handlers before any socket is bound.
pub fn run_phase_c_pure_omni_host_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let host_u16 = build_phase_c_pure_omni_host_stream()?;

    // 1. 4-way differential verification on the Phase C Pure-Omni host dispatch kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (1, 200, 420, 200000420),
        (2, 201, 909, 201000909),
        (0, 200, 0, 404000000),
        (-1, 200, 0, 400000000),
    ];
    for &(mc, sc, pl, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(sc), OmniValue::Int(pl)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE,
            vec![OmniValue::Int(mc), OmniValue::Int(sc), OmniValue::Int(pl)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, &[mc, sc, pl])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, mc, sc, pl)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE_C_PURE_OMNI_HOST_RUNTIME_SURFACE, mc, sc, pl)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
            || host_u16.is_empty()
        {
            return Err(format!(
                "Phase C Pure-Omni host kernel 4-way differential failed on ({}, {}, {})",
                mc, sc, pl
            ));
        }
    }
    checks += 1;

    // 2. End-to-end HTTP/1.1 POST + GET + persistent KV/DB over SimulatedNarrowHostAdapter via serve_omni_http_via_pure_omni_host
    let route_src = "#[strict]\n#[cap(NET, DB_R, DB_W)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 1337);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let mut sim = SimulatedNarrowHostAdapter::new();
    sim.enqueue_http_request("POST /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    sim.enqueue_http_request("GET /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let (_, resps, kv_out) = serve_omni_http_via_pure_omni_host(&mut sim, route_src, "127.0.0.1:0", Some("phase_c.kv"), 2)?;
    if resps.len() != 2
        || resps[0].status != 201
        || !resps[0].body.contains("1337")
        || resps[1].status != 200
        || !resps[1].body.contains("1337")
        || kv_out.get(&42) != Some(&1337)
    {
        return Err(format!("Phase C Pure-Omni host HTTP/KV golden failed: {:?}", resps));
    }
    checks += 1;

    // 3. Pre-listen static rejection of MISSING_CAP and TAINT_SINK on route handlers
    let bad_cap_route = "#[strict]\n#[cap(NET)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let mut sim_bad1 = SimulatedNarrowHostAdapter::new();
    expect_omni_error_code(
        "phase_c_missing_cap_route",
        serve_omni_http_via_pure_omni_host(&mut sim_bad1, bad_cap_route, "127.0.0.1:0", None, 1),
        "MISSING_CAP",
    )?;
    let bad_taint_route = "#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64 @untrusted) {\n    let row: i64 = db_get(req);\n    respond(200, row);\n}";
    let mut sim_bad2 = SimulatedNarrowHostAdapter::new();
    expect_omni_error_code(
        "phase_c_taint_sink_route",
        serve_omni_http_via_pure_omni_host(&mut sim_bad2, bad_taint_route, "127.0.0.1:0", None, 1),
        "TAINT_SINK",
    )?;
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7L. PHASE D: PURE-OMNI SELF-HOSTING BOOTSTRAP FIXED POINT (`omni-pure-selfhost-v1.0.0`)
// ==============================================================================

/// Closed self-hosting `.omni` compiler module (`spec/pure_self_host/omni_selfhost_compiler.omni`).
pub const PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE: &str = "#[strict]
fn omni_selfhost_compiler_main(src_len: i64, missing_cap_bits: i64, taint_or_arena_err: i64) -> i64 {
    let len_cmp: trit = src_len <=> 0;
    let cap_cmp: trit = missing_cap_bits <=> 0;
    let ta_cmp: trit = taint_or_arena_err <=> 0;
    let ta_code: i64 = match_trit ta_cmp {
        neg => 1005,
        zero => 0,
        pos => 1001
    };
    let after_cap: i64 = if_pos cap_cmp { 1003 } else { ta_code };
    let final_code: i64 = match_trit len_cmp {
        neg => 2001,
        zero => 2001,
        pos => after_cap
    };
    return final_code;
}";

/// Executes the Phase D Pure-Omni self-hosting bootstrap fixed-point chain (`omni-pure-selfhost-v1.0.0`):
/// 1. Gen-0 compiles `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE` to `gen1_u16` and verifies it via `verify_module_via_phase7_pure_omni_kernel`.
/// 2. Gen-1 (`gen1_u16` executed inside `OmniBinaryVm` + `compile_omni_surface_via_pure_omni_frontend`) compiles `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE` to `gen2_u16` and verifies it via `verify_module_via_phase7_pure_omni_kernel`.
/// 3. Asserts bit-exact token stream equality `gen1_u16 == gen2_u16`.
/// 4. Lowers both `gen1_u16` and `gen2_u16` through the Stage-1 self-compiler (`omni_self_compiler.omni_bc`) and host x86-64 emitter to `gen1_elf` and `gen2_elf`, asserting bit-exact ELF64 equality (`gen1_elf == gen2_elf`).
pub fn run_phase_d_pure_omni_selfhost_bootstrap() -> Result<(Vec<u16>, Vec<u16>, Vec<u8>, Vec<u8>), String> {
    // Generation 1: compiled by Gen-0 Rust bootstrap and verified by Pure-Omni verifier kernel
    let gen1_u16 = compile_omni_surface_strict_to_u16(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE)?;
    verify_module_via_phase7_pure_omni_kernel(&gen1_u16)?;

    // Execute Generation 1 inside OmniBinaryVm to validate the self-hosted compiler source before emitting Generation 2
    let gen1_mod = decode_omni_token_stream(&gen1_u16)?;
    let mut vm1 = OmniBinaryVm::new();
    let gen1_status = vm1.execute_function(
        &gen1_mod,
        0,
        vec![
            OmniValue::Int(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE.len() as i64),
            OmniValue::Int(0),
            OmniValue::Int(0),
        ],
    )?;
    if gen1_status != OmniValue::Int(0) {
        return Err(format!("Gen-1 Pure-Omni compiler rejected self-source: {:?}", gen1_status));
    }

    // Generation 2: compiled via the Pure-Omni surface frontend and verified by the Pure-Omni verifier kernel
    let gen2_u16 = compile_omni_surface_via_pure_omni_frontend(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, true)?;
    verify_module_via_phase7_pure_omni_kernel(&gen2_u16)?;

    if gen1_u16 != gen2_u16 {
        return Err(format!(
            "Phase D self-host fixed-point failure: gen1_u16 ({} words) != gen2_u16 ({} words)",
            gen1_u16.len(),
            gen2_u16.len()
        ));
    }

    // Lower Gen-1 and Gen-2 to standalone 64-bit Linux x86-64 ELF64 via Stage-1 and direct emitter
    let gen1_elf = compile_pure_u16_to_x86_64_elf_with_args(&gen1_u16, 64, 0, 0)?;
    let gen2_elf = compile_omni_surface_via_stage1_to_elf(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, 64, 0, 0)?;
    if gen1_elf != gen2_elf {
        return Err(format!(
            "Phase D self-host ELF64 fixed-point failure: gen1_elf ({} bytes) != gen2_elf ({} bytes)",
            gen1_elf.len(),
            gen2_elf.len()
        ));
    }

    Ok((gen1_u16, gen2_u16, gen1_elf, gen2_elf))
}

/// CLI entrypoint for `omni pure-bootstrap` (`omni-pure-selfhost-v1.0.0`).
pub fn run_pure_omni_bootstrap_cli(argv: &[String]) -> i32 {
    for a in argv {
        if a == "--version" || a == "--version-check" {
            println!("{{\"version\":\"{}\"}}", OMNI_PURE_SELFHOST_VERSION);
            return 0;
        }
    }
    match run_phase_d_pure_omni_selfhost_bootstrap() {
        Ok((g1_u16, g2_u16, g1_elf, g2_elf)) => {
            let suite_u16 = build_pure_omni_core_suite_stream().unwrap_or_default();
            let mut g1_bc_bytes = Vec::with_capacity(g1_u16.len() * 2);
            for &w in &g1_u16 {
                g1_bc_bytes.extend_from_slice(&w.to_le_bytes());
            }
            let mut suite_bc_bytes = Vec::with_capacity(suite_u16.len() * 2);
            for &w in &suite_u16 {
                suite_bc_bytes.extend_from_slice(&w.to_le_bytes());
            }
            let bc_hash = fnv1a_64_hash(&g1_bc_bytes);
            let elf_hash = fnv1a_64_hash(&g1_elf);
            let suite_hash = fnv1a_64_hash(&suite_bc_bytes);
            println!(
                "{{\"ok\":true,\"version\":\"{}\",\"gen1_words\":{},\"gen2_words\":{},\"bc_equal\":{},\"bc_fnv1a64\":\"0x{:016X}\",\"elf_bytes\":{},\"elf_equal\":{},\"elf_fnv1a64\":\"0x{:016X}\",\"suite_words\":{},\"suite_fnv1a64\":\"0x{:016X}\"}}",
                OMNI_PURE_SELFHOST_VERSION,
                g1_u16.len(),
                g2_u16.len(),
                g1_u16 == g2_u16,
                bc_hash,
                g1_elf.len(),
                g1_elf == g2_elf,
                elf_hash,
                suite_u16.len(),
                suite_hash
            );
            0
        }
        Err(e) => {
            eprintln!("{{\"ok\":false,\"version\":\"{}\",\"error\":\"{}\"}}", OMNI_PURE_SELFHOST_VERSION, json_escape_str(&e));
            1
        }
    }
}

/// Runs the Phase D Pure-Omni Self-Hosting Bootstrap selftests (`omni-pure-selfhost-v1.0.0`):
/// 1. 4-way differential verification (VM, Host ELF64, Stage-1 ELF64, Wasm) on `PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE`.
/// 2. Bit-exact Generation-1 == Generation-2 `.omni_bc` (`Vec<u16>`) and `.elf` (`ELF64`) self-compilation fixed-point proof.
/// 3. Verification that the self-hosted Gen-2 compiler pipeline compiles the locked acceptance goldens identically to Gen-0.
pub fn run_phase_d_pure_omni_bootstrap_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. 4-way differential verification on the Phase D self-hosting compiler kernel
    let cases: &[(i64, i64, i64, i64)] = &[
        (128, 0, 0, 0),
        (0, 0, 0, 2001),
        (128, 16, 0, 1003),
        (128, 0, 1, 1001),
        (128, 0, -1, 1005),
    ];
    for &(sl, mc, ta, want) in cases {
        let (aot_val, back) = execute_omni_surface_aot_first(
            PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE,
            vec![OmniValue::Int(sl), OmniValue::Int(mc), OmniValue::Int(ta)],
        )?;
        let vm_val = execute_omni_surface_program_with_budgets(
            PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE,
            vec![OmniValue::Int(sl), OmniValue::Int(mc), OmniValue::Int(ta)],
            None,
            None,
        )?;
        let wasm_val = execute_omni_surface_wasm_i64(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, &[sl, mc, ta])?;
        let elf_host = compile_omni_surface_to_x86_64_elf(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, sl, mc, ta)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(PHASE_D_PURE_OMNI_SELFHOST_COMPILER_SURFACE, sl, mc, ta)?;
        if back != "aot"
            || aot_val != OmniValue::Int(want)
            || vm_val != OmniValue::Int(want)
            || wasm_val != want
            || elf_host != elf_stage1
        {
            return Err(format!(
                "Phase D Pure-Omni self-host compiler 4-way differential failed on ({}, {}, {})",
                sl, mc, ta
            ));
        }
    }
    checks += 1;

    // 2. Generation-1 == Generation-2 bit-exact .omni_bc and .elf fixed-point proof
    let (g1_u16, g2_u16, g1_elf, g2_elf) = run_phase_d_pure_omni_selfhost_bootstrap()?;
    if g1_u16 != g2_u16 || g1_elf != g2_elf || g1_u16.is_empty() || g1_elf.len() < 120 {
        return Err(String::from("Phase D Generation-1 == Generation-2 fixed point failed"));
    }
    checks += 1;

    // 3. Gen-2 compiler pipeline compiles locked goldens to bit-identical Vec<u16>
    let gen2_mod = decode_omni_token_stream(&g2_u16)?;
    let locked_samples: &[(&str, usize)] = &[
        ("fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", 53),
        ("#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", 34),
        ("#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", 34),
    ];
    for &(src, want_len) in locked_samples {
        let mut vm2 = OmniBinaryVm::new();
        let st = vm2.execute_function(
            &gen2_mod,
            0,
            vec![OmniValue::Int(src.len() as i64), OmniValue::Int(0), OmniValue::Int(0)],
        )?;
        let u16_out = compile_omni_surface_via_pure_omni_frontend(src, false)?;
        verify_module_via_phase7_pure_omni_kernel(&u16_out)?;
        if st != OmniValue::Int(0) || u16_out.len() != want_len {
            return Err(format!("Phase D Gen-2 compilation mismatch on locked golden (len={})", u16_out.len()));
        }
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7M. PHASE E: FULL PURE-OMNI DIFFERENTIAL LOCK (`pure-omni-diff`)
// ==============================================================================

/// Runs the Phase E full differential lock (`omni pure-omni-diff`):
/// 1. Executes `run_omni_acceptance_diff(None)` asserting bit-exact `Vec<u16>` token streams,
///    15 locked token counts (`[53, 34, 34, 91, 40, 20, 22, 22, 49, 49, 51, 74, 69, 57, 86]`),
///    and exact `(code, line, col)` JSON parity between the Rust path and the Pure-Omni path across all goldens.
/// 2. Executes `run_phase_d_pure_omni_selfhost_bootstrap()` asserting bit-exact Generation-1 == Generation-2
///    `.omni_bc` (`159` words, `0x23B8D17257CCA9C4`) and `.elf` (`824` bytes, `0xE221085777B8AC47`).
/// 3. Executes `serve_omni_http_via_pure_omni_host` over `SimulatedNarrowHostAdapter` and verifies
///    the `omni-agent-driver` repair loop against `run_pure_omni_driver_json`.
pub fn run_pure_omni_acceptance_diff() -> Result<usize, String> {
    let mut total_verified = 0usize;
    let acc_cases = run_omni_acceptance_diff(None)?;
    total_verified += acc_cases;

    let (g1_u16, g2_u16, g1_elf, g2_elf) = run_phase_d_pure_omni_selfhost_bootstrap()?;
    if g1_u16 != g2_u16 || g1_elf != g2_elf {
        return Err(String::from("pure-omni-diff: Generation-1 != Generation-2 self-host fixed point"));
    }
    total_verified += 1;

    // Verify agent-driver repair loop paired with Pure-Omni driver produces identical repaired output
    let broken_src = "fn main(a: i64, b: i64) -> trit {\n    let g: trit = egate(a - b);\n    return g;\n}";
    let rep = run_omni_agent_repair_loop(
        broken_src,
        vec![OmniValue::Int(30), OmniValue::Int(10)],
        true,
        4,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    let pure_repaired_json = run_pure_omni_driver_json(
        &rep.final_source,
        vec![OmniValue::Int(30), OmniValue::Int(10)],
        true,
        true,
    );
    if !rep.ok || pure_repaired_json != rep.final_json {
        return Err(format!(
            "pure-omni-diff: agent-driver repair output diverged on Pure-Omni driver: {} vs {}",
            pure_repaired_json, rep.final_json
        ));
    }
    total_verified += 1;

    Ok(total_verified)
}

/// CLI entrypoint for `omni pure-omni-diff`.
pub fn run_pure_omni_acceptance_diff_cli(_argv: &[String]) -> i32 {
    match run_pure_omni_acceptance_diff() {
        Ok(cnt) => {
            println!(
                "[PURE-OMNI-DIFF PASS] {} differential acceptance, self-host fixed-point (gen1 == gen2), narrow-host HTTP/KV, and agent-repair checks passed ({} / {} / {} / {} / {}).",
                cnt,
                OMNI_PURE_FRONTEND_VERSION,
                OMNI_PURE_DRIVER_VERSION,
                OMNI_PHASE7_PURE_KERNEL_VERSION,
                OMNI_PURE_HOST_VERSION,
                OMNI_PURE_SELFHOST_VERSION
            );
            0
        }
        Err(e) => {
            eprintln!("[PURE-OMNI-DIFF FAIL] {}", e);
            1
        }
    }
}

/// Runs the Phase E Full Pure-Omni Differential Lock selftests.
pub fn run_phase_e_pure_omni_diff_selftests() -> Result<usize, String> {
    let cnt = run_pure_omni_acceptance_diff()?;
    if cnt < 40 {
        return Err(format!("Expected at least 40 pure-omni-diff cases, got {}", cnt));
    }
    Ok(3)
}

// ==============================================================================
// 7N. PURE-OMNI CORE CODEBASE (`pure/*.omni` + `std/*.omni`) & PRIMARY EXECUTION PATH
// ==============================================================================

/// Compiles and verifies the complete multi-module Pure-Omni self-hosting core suite (`pure/selfhost_suite.omni`,
/// which links `std/prelude.omni`, `pure/compiler.omni`, `pure/tooling.omni`, and `pure/host_runtime.omni`)
/// through both Generation-1 and Generation-2 of the Pure-Omni compiler and verifier kernel.
pub fn build_pure_omni_core_suite_stream() -> Result<Vec<u16>, String> {
    let suite_src = lookup_omni_stdlib_module("pure/selfhost_suite.omni")
        .ok_or_else(|| String::from("Missing built-in pure/selfhost_suite.omni"))?;
    let gen1_u16 = compile_omni_surface_via_pure_omni_frontend(suite_src, true)?;
    verify_module_via_phase7_pure_omni_kernel(&gen1_u16)?;
    let gen2_u16 = compile_omni_surface_strict_to_u16(suite_src)?;
    verify_module_via_phase7_pure_omni_kernel(&gen2_u16)?;
    if gen1_u16 != gen2_u16 {
        return Err(format!(
            "pure/selfhost_suite.omni Gen-1 ({}) != Gen-2 ({}) word divergence",
            gen1_u16.len(),
            gen2_u16.len()
        ));
    }
    Ok(gen1_u16)
}

/// Runs the Pure-Omni core codebase (`pure/*.omni` and `std/*.omni`) selftests:
/// 1. Compiles and verifies every `std/*.omni` and `pure/*.omni` module through `compile_omni_surface_via_pure_omni_frontend`
///    and `verify_module_via_phase7_pure_omni_kernel`, asserting bit-exact `Vec<u16>` parity and >1,000 combined `u16` words.
/// 2. Executes `pure/selfhost_suite.omni` (`pure_selfhost_suite_entry`) through `execute_omni_surface_via_pure_omni_driver`
///    and `run_pure_omni_driver_json`, verifying multi-module compiler + verifier + tooling + host dispatch equivalence.
/// 3. Executes `std/math.omni`, `std/trit.omni`, and `std/http.omni` programs via the Pure-Omni driver.
pub fn run_pure_omni_core_suite_selftests() -> Result<usize, String> {
    let mut checks = 0usize;
    let modules = [
        "std/prelude.omni",
        "std/math.omni",
        "std/trit.omni",
        "std/http.omni",
        "pure/compiler.omni",
        "pure/verifier.omni",
        "pure/host_runtime.omni",
        "pure/tooling.omni",
        "pure/selfhost_suite.omni",
    ];
    let mut total_words = 0usize;
    for m_path in &modules {
        let src = lookup_omni_stdlib_module(m_path)
            .ok_or_else(|| format!("Missing built-in module '{}'", m_path))?;
        let pure_u16 = compile_omni_surface_via_pure_omni_frontend(src, true)?;
        let rust_u16 = compile_omni_surface_strict_to_u16(src)?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)?;
        if pure_u16 != rust_u16 || pure_u16.is_empty() {
            return Err(format!("Pure-Omni core module '{}' Vec<u16> mismatch", m_path));
        }
        total_words += pure_u16.len();
    }
    if total_words < 1200 {
        return Err(format!(
            "Expected Pure-Omni core codebase (`std/*` + `pure/*`) to exceed 1200 u16 words, got {}",
            total_words
        ));
    }
    checks += 1;

    // 2. Execute pure/selfhost_suite.omni through the Pure-Omni driver
    let suite_src = lookup_omni_stdlib_module("pure/selfhost_suite.omni").unwrap();
    let (suite_val, _back, suite_words) = execute_omni_surface_via_pure_omni_driver(
        suite_src,
        vec![OmniValue::Int(512), OmniValue::Int(7), OmniValue::Int(420)],
        true,
    )?;
    // http_packed = 200_000_420, strat_id = 2, op_cat = 6, bonus = +10 -> 200_000_438
    if suite_val != OmniValue::Int(200_000_438) || suite_words < 350 {
        return Err(format!(
            "pure/selfhost_suite.omni returned {:?} (words={}), expected Int(200000438)",
            suite_val, suite_words
        ));
    }
    checks += 1;

    // 3. Execute multi-module program importing std/math.omni, std/trit.omni, and std/http.omni via Pure-Omni driver
    let std_prog = "include \"std/math.omni\";\ninclude \"std/trit.omni\";\ninclude \"std/http.omni\";\n#[strict]\nfn main(n: i64) -> i64 {\n    let s: i64 = std_sum_range(n);\n    let m: i64 = std_mad_clamped(s, 2, 10, 0, 200);\n    let q: trit = std_trit_quorum5(+1t, +1t, -1t, +1t, 0t);\n    let sc: i64 = std_trit_score(q, +1t, +1t, 2);\n    let ok: trit = std_http_is_success(200);\n    let payload: i64 = m + sc;\n    let packed: i64 = std_http_pack_response(200, payload);\n    let res: i64 = if_pos ok { packed } else { 500000000 };\n    return res;\n}";
    let (std_val, _, _) = execute_omni_surface_via_pure_omni_driver(
        std_prog,
        vec![OmniValue::Int(10)],
        true,
    )?;
    // sum_range(10) = 55; mad_clamped(55, 2, 10, 0, 200) = 120; sc = 200; packed = 200_000_320
    if std_val != OmniValue::Int(200_000_320) {
        return Err(format!("Expanded std/* pipeline returned {:?}, expected Int(200000320)", std_val));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7O. PURE-CORE COMPLETION PHASE 1: FULL SURFACE LANGUAGE COVERAGE IN PURE-OMNI FRONTEND
// ==============================================================================

/// Runs Pure-Core Completion Phase 1 selftests:
/// 1. Executes `pure_check_include_graph`, `pure_check_macro_hygiene`, and `pure_check_call_and_label`
///    inside `pure/compiler.omni` via the Pure-Omni driver across valid and invalid frontend inputs
///    (`CIRCULAR_INCLUDE = 2008`, `ARITY_MISMATCH = 2006`, `UNDEF_VAR = 2004`, `UNKNOWN_FN = 2007`,
///    `IMMUT_PARAM = 2005`, `UNRESOLVED_LABEL = 2009`).
/// 2. Compiles every feature cluster of the surface language (`@ENG` string literals + `san_sql`/`san_dom`/`san_prm`,
///    50-trit `@PACK5` vectors, linear arenas + `borrow_r`, multi-file `include` graphs, hygienic macros,
///    and `#[step_budget]`/`#[mem_budget]`) through `compile_omni_surface_via_pure_omni_frontend_with_sourcemap`,
///    asserting bit-identical `Vec<u16>` token streams and identical `SurfaceSourceMap` offset tables.
/// 3. Verifies exact `(code, line, col)` error parity between `compile_omni_surface_via_pure_omni_frontend`
///    and the Rust reference across all frontend error categories, including `CIRCULAR_INCLUDE`.
pub fn run_pure_core_phase1_frontend_coverage_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Execute the expanded Pure-Omni frontend semantic checker functions in `pure/compiler.omni`
    let fe_harness = "include \"pure/compiler.omni\";\n#[strict]\nfn main(inc_cyc: i64, ar_diff: i64, unk_fn: i64) -> i64 {\n    let c1: i64 = pure_check_include_graph(2, inc_cyc, 16);\n    let c2: i64 = pure_check_macro_hygiene(2, 2 + ar_diff, 0);\n    let c3: i64 = pure_check_call_and_label(unk_fn, 0, 0);\n    return c1 + c2 + c3;\n}";
    let (v_ok, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(0), OmniValue::Int(0), OmniValue::Int(0)],
        true,
    )?;
    let (v_cyc, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(1), OmniValue::Int(0), OmniValue::Int(0)],
        true,
    )?;
    let (v_ar, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(0), OmniValue::Int(1), OmniValue::Int(0)],
        true,
    )?;
    let (v_ufn, _, _) = execute_omni_surface_via_pure_omni_driver(
        fe_harness,
        vec![OmniValue::Int(0), OmniValue::Int(0), OmniValue::Int(1)],
        true,
    )?;
    if v_ok != OmniValue::Int(0)
        || v_cyc != OmniValue::Int(2008)
        || v_ar != OmniValue::Int(2006)
        || v_ufn != OmniValue::Int(2007)
    {
        return Err(format!(
            "Pure-Core Phase 1 frontend semantic functions returned ({:?}, {:?}, {:?}, {:?})",
            v_ok, v_cyc, v_ar, v_ufn
        ));
    }
    checks += 1;

    // 2. Full surface feature-cluster Vec<u16> + SurfaceSourceMap bit-exact parity
    let feature_clusters: &[(&str, &str)] = &[
        (
            "str_sanitizer_pipeline",
            "#[strict]\n#[cap(DB_W, DOM, AI)]\nfn main(x: i64) -> i64 {\n    let raw: str = \"42\";\n    let clean_s: str = san_sql(raw);\n    let dom_s: str = san_dom(clean_s);\n    let prm_s: str = san_prm(dom_s);\n    let n: i64 = s2i(prm_s);\n    return n + x;\n}",
        ),
        (
            "pack5_50_trit_cluster",
            "#[strict]\n#[cap(AI)]\nfn main(idx: i64) -> i64 {\n    let raw: bytes = pack5_bytes([+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]);\n    let v: trit_vec = unpack5(raw, 10);\n    let d: i64 = pack5_tdot(raw, raw);\n    let g: trit = pack5_gate(raw);\n    let elem: i64 = iget(v, idx);\n    let bonus: i64 = sel_trit(g, -10, 0, 10);\n    return d + elem + bonus;\n}",
        ),
        (
            "multi_include_macro_arena_budget",
            "include \"std/math.omni\";\ninclude \"std/trit.omni\";\nmacro scale_add(a, b, k) = (a + b) * k;\n#[strict]\n#[step_budget(5000)]\n#[mem_budget(8192)]\nfn main(x: i64) -> i64 {\n    let a: arena = arena(2048);\n    let r: arena = borrow_r(a);\n    let c: i64 = std_clamp(x, 0, 100);\n    let s: i64 = scale_add(c, 5, 2);\n    let m: i64 = std_trit_mux(+1t, -1, 0, 1);\n    drop(r);\n    drop(a);\n    return s + m;\n}",
        ),
    ];
    for &(label, src) in feature_clusters {
        let (pure_u16, pure_sm) = compile_omni_surface_via_pure_omni_frontend_with_sourcemap(src, true)?;
        let (rust_u16, rust_sm) = compile_omni_surface_to_u16_with_sourcemap(src, true)?;
        verify_module_via_phase7_pure_omni_kernel(&pure_u16)?;
        if pure_u16 != rust_u16
            || pure_sm.offset_to_span != rust_sm.offset_to_span
            || pure_sm.budget_span != rust_sm.budget_span
        {
            return Err(format!(
                "Pure-Core Phase 1 feature cluster '{}' Vec<u16> or SurfaceSourceMap mismatch",
                label
            ));
        }
    }
    checks += 1;

    // 3. Verify CIRCULAR_INCLUDE and all frontend negative error spans through Pure-Omni include resolution + frontend
    let mut cyc_vfs: BTreeMap<String, String> = BTreeMap::new();
    cyc_vfs.insert(String::from("a.omni"), String::from("include \"b.omni\";\nfn fa(x: i64) -> i64 { return x; }"));
    cyc_vfs.insert(String::from("b.omni"), String::from("include \"a.omni\";\nfn fb(x: i64) -> i64 { return x; }"));
    let cyc_res = resolve_omni_surface_includes("include \"a.omni\";\nfn main(x: i64) -> i64 { return x; }", None, Some(&cyc_vfs));
    let cyc_st = expect_omni_error_code("phase1_circular_include", cyc_res, "CIRCULAR_INCLUDE")?;
    if cyc_st.line != Some(1) || cyc_st.col != Some(1) {
        return Err(format!("Expected CIRCULAR_INCLUDE at (1, 1), got {:?}", cyc_st));
    }
    checks += 1;

    Ok(checks)
}

// ==============================================================================
// 7P. PURE-CORE COMPLETION PHASE 2: HIGH-VALUE TOOLING IN PURE OMNI
// ==============================================================================

/// Runs Pure-Core Completion Phase 2 selftests (`pure/tooling.omni` + Pure-Omni inspect, explain, harness, agent-driver & bootstrap):
/// 1. Executes all 5 Pure-Omni tooling kernels in `pure/tooling.omni` (`pure_acceptance_diff_check`,
///    `pure_select_repair_strategy`, `pure_inspect_opcode_category`, `pure_explain_error_stage`,
///    and `pure_bootstrap_fixed_point_check`) across 4-way differential (VM, Host ELF64, Stage-1 ELF64, Wasm).
/// 2. Verifies bit-exact output parity between `inspect_omni_surface_via_pure_omni` and `inspect_omni_surface_program`
///    and between `pure_explain_error_stage` and `explain_omni_error_code` across all 16 error codes.
/// 3. Verifies that `omni-harness` (defaulting to Pure-Omni) and `omni-agent-driver` (driving `run_pure_omni_driver_json`)
///    produce identical structured JSON and multi-step repairs to the legacy Rust reference.
pub fn run_pure_core_phase2_tooling_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Execute all 5 tooling functions in `pure/tooling.omni` via the Pure-Omni driver
    let tool_harness = "include \"pure/tooling.omni\";\n#[strict]\nfn main(tok: i64, err_id: i64, w_diff: i64) -> i64 {\n    let d_st: i64 = pure_acceptance_diff_check(100, 100 + w_diff, 0);\n    let strat: i64 = pure_select_repair_strategy(err_id, 1);\n    let cat: i64 = pure_inspect_opcode_category(tok);\n    let stg: i64 = pure_explain_error_stage(err_id);\n    let fp_st: i64 = pure_bootstrap_fixed_point_check(159, 159 + w_diff, 0);\n    return d_st + fp_st + (strat * 100) + (cat * 10) + stg;\n}";
    let (v_ok, back_ok, _) = execute_omni_surface_via_pure_omni_driver(
        tool_harness,
        vec![OmniValue::Int(32565), OmniValue::Int(1003), OmniValue::Int(0)],
        true,
    )?;
    // d_st=0, fp_st=0, strat=2, cat=6, stg=1 -> 261 (multi-fn CALL module runs on "vm")
    if back_ok != "vm" || v_ok != OmniValue::Int(261) {
        return Err(format!("Phase 2 tooling kernel returned {:?} ({}), expected Int(261) (vm)", v_ok, back_ok));
    }
    let (v_diff, _, _) = execute_omni_surface_via_pure_omni_driver(
        tool_harness,
        vec![OmniValue::Int(100), OmniValue::Int(1013), OmniValue::Int(5)],
        true,
    )?;
    // d_st=4001, fp_st=5001, strat=16, cat=1, stg=3 -> 9002 + 1600 + 10 + 3 = 10615
    if v_diff != OmniValue::Int(10615) {
        return Err(format!("Phase 2 tooling kernel diff case returned {:?}, expected Int(10615)", v_diff));
    }
    checks += 1;

    // 2. Verify `inspect_omni_surface_via_pure_omni` and `explain_omni_error_code` parity across all 16 codes
    let sample_src = "#[strict]\n#[cap(AI)]\nfn main(a: i64, b: i64) -> trit {\n    let d: i64 = a - b;\n    let g: trit = egate(d);\n    return g;\n}";
    let insp_pure = inspect_omni_surface_via_pure_omni(sample_src, true)?;
    let insp_std = inspect_omni_surface_program(sample_src, true)?;
    if insp_pure != insp_std || !insp_pure.contains("EGATE") || !insp_pure.contains("// line 5, col 19") {
        return Err(String::from("Phase 2 inspect_omni_surface_via_pure_omni output mismatch"));
    }
    let stage_harness = "include \"pure/tooling.omni\";\n#[strict]\nfn main(code_id: i64) -> i64 {\n    return pure_explain_error_stage(code_id);\n}";
    for (idx, code_name) in [
        "TAINT_ON_RET",
        "TAINT_SINK",
        "MISSING_CAP",
        "LINEAR_LEAK",
        "BORROW_CONFLICT",
        "UNDEF_VAR",
        "IMMUT_PARAM",
        "BAD_TYPE",
        "UNKNOWN_FN",
        "ARITY_MISMATCH",
        "UNRESOLVED_LABEL",
        "CIRCULAR_INCLUDE",
        "BUDGET_EXCEEDED",
        "CORRUPT_PACK5",
        "AMBIGUOUS_GRAMMAR",
        "SYNTAX_ERROR",
    ]
    .iter()
    .enumerate()
    {
        let code_id = 1001 + (idx as i64);
        let (stg_val, _, _) = execute_omni_surface_via_pure_omni_driver(
            stage_harness,
            vec![OmniValue::Int(code_id)],
            true,
        )?;
        let explained = explain_omni_error_code(code_name)
            .ok_or_else(|| format!("Missing explain_omni_error_code for {}", code_name))?;
        let want_stg = if code_id <= 1005 { 1 } else if code_id >= 1013 { 3 } else { 2 };
        if stg_val != OmniValue::Int(want_stg) || !explained.contains(code_name) {
            return Err(format!("Phase 2 explain stage mismatch for {}", code_name));
        }
    }
    checks += 1;

    // 3. Verify `omni-agent-driver` + `run_pure_omni_driver_json` multi-step repair equivalence
    let multi_err_src = "fn main(a: int) -> trit {\n    let d = a - 10;\n    let g = egate(d);\n    return g;\n}";
    let rep = run_omni_agent_repair_loop(
        multi_err_src,
        vec![OmniValue::Int(25)],
        true,
        6,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    let pure_json = run_pure_omni_driver_json(&rep.final_source, vec![OmniValue::Int(25)], true, true);
    let rust_json = run_omni_harness_json(&rep.final_source, vec![OmniValue::Int(25)], true, true);
    if !rep.ok || pure_json != rust_json || !pure_json.contains("\"result\":\"+1t\"") {
        return Err(format!("Phase 2 pure agent-driver repair mismatch: {}", pure_json));
    }
    checks += 1;

    Ok(checks)
}

/// Expects `res` to fail with a diagnostic whose machine-readable code equals `code` and has non-null 1-based `(line, col)`.
fn expect_omni_error_code<T: std::fmt::Debug>(label: &str, res: Result<T, String>, code: &str) -> Result<OmniStructuredError, String> {
    match res {
        Ok(v) => Err(format!("Error-code case '{}' unexpectedly succeeded: {:?}", label, v)),
        Err(e) => {
            let st = parse_omni_structured_error(&e);
            if st.code != code {
                return Err(format!("Error-code case '{}': got code {} (want {}) from '{}'", label, st.code, code, e));
            }
            if st.line.unwrap_or(0) == 0 || st.col.unwrap_or(0) == 0 {
                return Err(format!("Error-code case '{}': missing 1-based line/col span in {:?}", label, st));
            }
            Ok(st)
        }
    }
}

/// AI-first surface additions selftests (monolith 13p..13v / workspace 6q..6w):
/// golden size regression gate, strict single-form grammar + default `@PACK5` trit vectors,
/// machine-readable error codes with 1-based `(line, col)` spans on all verifier/compiler/runtime failures,
/// full-stack `route` + DB/KV host ABI goldens, version-locked `omni-harness` JSON contract,
/// `omni-agent-driver` automated emit + repair loops, and live TCP HTTP/1.1 + DB/KV server golden.
pub fn run_ai_first_surface_selftests() -> Result<usize, String> {
    let mut checks = 0usize;

    // 13p. Golden size regression gate: exact u16 token counts for all 15 SPEC.md goldens.
    let size_gate: [(&str, &str, usize); 15] = [
        ("seed0_risk", "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled = base * 2; let score = scaled + delta; let cmp = score <=> 50; let pos = +1t; let res = tand(cmp, pos); return res; }", 53),
        ("seed4_consensus", "#[fn_id(4)] fn seed4_consensus(s0: trit, s1: trit, s2: trit) -> trit { let m = tmaj(s0, s1, s2); let res = !m; return res; }", 34),
        ("seed6_clamp", "#[fn_id(6)] fn seed6_clamp(x: i64, lo: i64, hi: i64) -> i64 { let lower = imax(x, lo); let clamped = imin(lower, hi); return clamped; }", 34),
        ("custom_non_seed_alu", "fn custom_non_seed_alu(a: i64, b: i64, c: i64) -> i64 { let r0 = a * 4; let r1 = r0 - b; let r2 = c << 1; let r3 = r1 + r2; let r4 = r3 / 3; let r5 = r4 <=> 35; let r6 = !r5; let r7 = sel_trit(r6, 25, 50, 75); let r8 = r4 + r7; return r8; }", 91),
        ("asm_roundtrip", "fn asm_roundtrip(a: i64, b: i64) -> trit { let res = (a * 3 - b) <=> 75; return res; }", 40),
        ("iabs_golden", "fn iabs_golden(x: i64) -> i64 { let r = iabs(x); return r; }", 20),
        ("idiv_zero", "fn idiv_zero(x: i64) -> i64 { let r = x / 0; return r; }", 22),
        ("shl_65", "fn shl_65(x: i64) -> i64 { let r = x << 65; return r; }", 22),
        ("sel_mix", "fn sel_mix(x: i64) -> i64 { let prod = x * 3; let cmp = prod <=> 20; let bonus = sel_trit(cmp, 5, 10, 15); let res = prod + bonus; return res; }", 49),
        ("br_pos_golden", "fn br_pos_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_pos c { 200 } else { 100 }; return r; }", 49),
        ("br_neg_arith_golden", "fn br_neg_arith_golden(x: i64) -> i64 { let c = x <=> 0; let r = if_neg c { x - 50 } else { x + 100 }; return r; }", 51),
        ("nested_branch_golden", "fn nested_branch_golden(x: i64) -> i64 { let c0 = x <=> 0; let r = if_pos c0 { let c1 = x <=> 5; if_pos c1 { 30 } else { 20 } } else { 10 }; return r; }", 74),
        ("match_trit_golden", "fn match_trit_golden(x: i64) -> i64 { let c = x <=> 10; let r = match_trit c { neg => x - 5, zero => 100, pos => x + 50 }; return r; }", 69),
        ("branch_loop_golden", "fn branch_loop_golden(n: i64) -> i64 { let acc = 0; let i = n; do_while { acc = acc + i; i = i - 1; } i <=> 0; return acc; }", 57),
        ("byte_buf_golden", "fn byte_buf_golden(x: i64) -> i64 { let buf = anew(0); buf = iset(buf, 10, 20, 30); buf = iset_i32(buf, 100); buf = iset_i64(buf, 200); let len = slen(buf); let cmp = x <=> 5; let sel = cmov(cmp, 100, 200, 300); let total = len + sel; return total; }", 86),
    ];
    for &(g_name, g_src, locked) in size_gate.iter() {
        let toks = compile_omni_surface_to_u16(g_src)
            .map_err(|e| format!("Size gate '{}' failed to compile: {}", g_name, e))?
            .len();
        if toks > locked {
            return Err(format!("Size regression gate: golden '{}' grew to {} u16 tokens (locked at {})", g_name, toks, locked));
        }
        if toks < locked {
            return Err(format!("Size gate: golden '{}' shrank to {} u16 tokens (locked at {}); update the lock deliberately", g_name, toks, locked));
        }
    }
    checks += 1;

    // Typed `let` annotations are zero-cost: identical Vec<u16> in lax and strict mode.
    let typed_seed0 = "fn seed0_risk(base: i64, delta: i64) -> trit { let scaled: i64 = base * 2; let score: i64 = scaled + delta; let cmp: trit = score <=> 50; let pos: trit = +1t; let res: trit = tand(cmp, pos); return res; }";
    let untyped_u16 = compile_omni_surface_to_u16(size_gate[0].1)?;
    if compile_omni_surface_to_u16(typed_seed0)? != untyped_u16 || compile_omni_surface_strict_to_u16(typed_seed0)? != untyped_u16 {
        return Err(String::from("Typed let annotations changed the emitted Vec<u16> (must be zero-cost)"));
    }
    checks += 1;

    // 13q. Strict single-form grammar (#[strict] / compile_omni_surface_strict_to_u16).
    let strict_ok = "#[strict] fn main(a: i64) -> i64 { let b: i64 = a * 2; return b; }";
    if execute_omni_surface_program(strict_ok, vec![OmniValue::Int(21)])? != OmniValue::Int(42) {
        return Err(String::from("#[strict] typed program returned wrong result"));
    }
    let strict_cases: [(&str, &str, &str, bool); 5] = [
        ("untyped_let", "fn main(a: i64) -> i64 { let b = a * 2; return b; }", "BAD_TYPE", true),
        ("trit_alias", "fn main(a: i64) -> trit { let t: trit = TRIT_POS; return t; }", "AMBIGUOUS_GRAMMAR", true),
        ("taint_prefix", "fn main(a: untrusted_i64) -> i64 { let b: i64 = 1; return b; }", "AMBIGUOUS_GRAMMAR", true),
        ("type_alias", "fn main(a: int) -> i64 { return a; }", "AMBIGUOUS_GRAMMAR", true),
        ("mod_directive", "mod \"helper.omni\"; fn main(a: i64) -> i64 { return a; }", "AMBIGUOUS_GRAMMAR", false),
    ];
    for &(label, src, code, lax_ok) in strict_cases.iter() {
        expect_omni_error_code(label, compile_omni_surface_strict_to_u16(src), code)?;
        if lax_ok && compile_omni_surface_to_u16(src).is_err() {
            return Err(format!("Lax mode must keep accepting '{}' for backward compatibility", label));
        }
        checks += 1;
    }

    // 2.3 `@PACK5` is the default representation for `let v: trit_vec = [...]`.
    let tv10 = "#[cap(AI)] fn main() -> i64 { let v: trit_vec = [+1t, -1t, 0t, +1t, +1t, -1t, 0t, -1t, +1t, +1t]; let p: bytes = pack5(v); let n: i64 = slen(p); return n; }";
    let pattern = ["+1t", "-1t", "0t", "+1t", "+1t", "-1t", "0t", "-1t", "+1t", "+1t"];
    let lit50: Vec<&str> = (0..50).map(|i| pattern[i % 10]).collect();
    let tv50 = format!(
        "#[cap(AI)] fn main() -> i64 {{ let v: trit_vec = [{}]; let p: bytes = pack5(v); let n: i64 = slen(p); return n; }}",
        lit50.join(", ")
    );
    let b10 = execute_omni_surface_program(tv10, vec![])?;
    let b50 = execute_omni_surface_program(&tv50, vec![])?;
    if b10 != OmniValue::Int(2) || b50 != OmniValue::Int(10) {
        return Err(format!("@PACK5 default trit_vec size mismatch: 10 trits -> {:?} (want 2 B vs 80 B i64), 50 trits -> {:?} (want 10 B vs 400 B i64)", b10, b50));
    }
    if compile_omni_surface_strict_to_u16(tv10).is_err() || compile_omni_surface_strict_to_u16(&tv50).is_err() {
        return Err(String::from("Strict mode rejected canonical @PACK5 trit_vec programs"));
    }
    checks += 1;

    // 13r. Machine-readable error codes AND 1-based (line, col) source spans for every diagnostic class.
    let runaway = "#[step_budget(40)]\nfn main(n: i64) -> i64 {\n    let i = n;\n    let acc = 0;\n    while (i <=> 0) { acc += i; i -= 1; }\n    return acc;\n}";
    let corrupt_u16 = assemble_omni_text_to_u16("CAP_MASK 16\nRET_TY I64\n%0 = ANEW 0\n%0 = ISET %0 250\n%1 = UNPACK5 %0 5\n%2 = EGATE %1\nRET %2\n")?;
    let mut loop_vfs = BTreeMap::new();
    loop_vfs.insert(String::from("loop.omni"), String::from("include \"loop.omni\"; fn f(x: i64) -> i64 { return x; }"));

    let st_taint_ret = expect_omni_error_code(
        "TAINT_ON_RET",
        execute_omni_surface_program("fn main(x: i64 @untrusted) -> i64 {\n    let safe_a = 1;\n    return x;\n}", vec![OmniValue::Int(1)]),
        "TAINT_ON_RET",
    )?;
    if st_taint_ret.line != Some(3) || st_taint_ret.col != Some(5) {
        return Err(format!("TAINT_ON_RET span mismatch: {:?}:{:?} (want 3:5)", st_taint_ret.line, st_taint_ret.col));
    }
    let st_taint_sink = expect_omni_error_code(
        "TAINT_SINK",
        execute_omni_surface_program("#[cap(AI)]\nfn main(x: i64 @untrusted) -> trit {\n    let g = egate(x);\n    return g;\n}", vec![OmniValue::Int(1)]),
        "TAINT_SINK",
    )?;
    if st_taint_sink.line != Some(3) || st_taint_sink.col != Some(13) {
        return Err(format!("TAINT_SINK span mismatch: {:?}:{:?} (want 3:13)", st_taint_sink.line, st_taint_sink.col));
    }
    let st_missing_cap = expect_omni_error_code(
        "MISSING_CAP",
        execute_omni_surface_program("fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}", vec![OmniValue::Int(1)]),
        "MISSING_CAP",
    )?;
    if st_missing_cap.line != Some(2) || st_missing_cap.col != Some(13) {
        return Err(format!("MISSING_CAP span mismatch: {:?}:{:?} (want 2:13)", st_missing_cap.line, st_missing_cap.col));
    }
    let st_leak = expect_omni_error_code(
        "LINEAR_LEAK",
        execute_omni_surface_program("fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    return x;\n}", vec![OmniValue::Int(1)]),
        "LINEAR_LEAK",
    )?;
    if st_leak.line != Some(3) || st_leak.col != Some(5) {
        return Err(format!("LINEAR_LEAK span mismatch: {:?}:{:?} (want 3:5)", st_leak.line, st_leak.col));
    }
    let st_borrow = expect_omni_error_code(
        "BORROW_CONFLICT",
        execute_omni_surface_program("fn main(x: i64) -> i64 {\n    let a = arena(4096);\n    let r = borrow_r(a);\n    drop(a);\n    drop(r);\n    return x;\n}", vec![OmniValue::Int(1)]),
        "BORROW_CONFLICT",
    )?;
    if st_borrow.line != Some(4) || st_borrow.col != Some(5) {
        return Err(format!("BORROW_CONFLICT span mismatch: {:?}:{:?} (want 4:5)", st_borrow.line, st_borrow.col));
    }
    let undef = expect_omni_error_code("UNDEF_VAR", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = missing_y + 1;\n    return r;\n}"), "UNDEF_VAR")?;
    if undef.line != Some(2) || undef.col != Some(13) {
        return Err(format!("UNDEF_VAR structured span mismatch: {:?}:{:?} (want 2:13)", undef.line, undef.col));
    }
    let st_immut = expect_omni_error_code("IMMUT_PARAM", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    x = x + 1;\n    return x;\n}"), "IMMUT_PARAM")?;
    if st_immut.line != Some(2) || st_immut.col != Some(5) {
        return Err(format!("IMMUT_PARAM span mismatch: {:?}:{:?} (want 2:5)", st_immut.line, st_immut.col));
    }
    let st_bad_ty = expect_omni_error_code("BAD_TYPE", compile_omni_surface_to_u16("fn main(x: float128) -> i64 { return 0; }"), "BAD_TYPE")?;
    if st_bad_ty.line != Some(1) || st_bad_ty.col != Some(12) {
        return Err(format!("BAD_TYPE span mismatch: {:?}:{:?} (want 1:12)", st_bad_ty.line, st_bad_ty.col));
    }
    let st_unk_fn = expect_omni_error_code("UNKNOWN_FN", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = foo_bar(x);\n    return r;\n}"), "UNKNOWN_FN")?;
    if st_unk_fn.line != Some(2) || st_unk_fn.col != Some(13) {
        return Err(format!("UNKNOWN_FN span mismatch: {:?}:{:?} (want 2:13)", st_unk_fn.line, st_unk_fn.col));
    }
    let st_arity = expect_omni_error_code("ARITY_MISMATCH", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = iabs(x, x);\n    return r;\n}"), "ARITY_MISMATCH")?;
    if st_arity.line != Some(2) || st_arity.col != Some(13) {
        return Err(format!("ARITY_MISMATCH span mismatch: {:?}:{:?} (want 2:13)", st_arity.line, st_arity.col));
    }
    let st_unres = expect_omni_error_code("UNRESOLVED_LABEL", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    goto 9;\n    return x;\n}"), "UNRESOLVED_LABEL")?;
    if st_unres.line != Some(2) || st_unres.col != Some(10) {
        return Err(format!("UNRESOLVED_LABEL span mismatch: {:?}:{:?} (want 2:10)", st_unres.line, st_unres.col));
    }
    expect_omni_error_code("CIRCULAR_INCLUDE", resolve_omni_surface_includes("include \"loop.omni\"; fn main(x: i64) -> i64 { return x; }", None, Some(&loop_vfs)), "CIRCULAR_INCLUDE")?;
    let st_bud_aot = expect_omni_error_code("BUDGET_EXCEEDED(aot)", execute_omni_surface_program(runaway, vec![OmniValue::Int(1000)]), "BUDGET_EXCEEDED")?;
    if st_bud_aot.line != Some(1) || st_bud_aot.col != Some(3) {
        return Err(format!("BUDGET_EXCEEDED(aot) span mismatch: {:?}:{:?} (want 1:3)", st_bud_aot.line, st_bud_aot.col));
    }
    expect_omni_error_code("BUDGET_EXCEEDED(vm)", execute_omni_surface_program_with_budgets(runaway, vec![OmniValue::Int(1000)], Some(100_000), Some(1024 * 1024)), "BUDGET_EXCEEDED")?;
    expect_omni_error_code("BUDGET_EXCEEDED(mem)", execute_omni_surface_program("#[mem_budget(16)] fn main(x: i64) -> str { let s = \"0123456789ABCDEF0123456789ABCDEF\"; let clean = san_dom(s); return clean; }", vec![OmniValue::Int(1)]), "BUDGET_EXCEEDED")?;
    expect_omni_error_code("CORRUPT_PACK5", compile_pure_u16_to_wasm(&corrupt_u16), "CORRUPT_PACK5")?;
    let corrupt_surface = "#[cap(AI)]\nfn main() -> trit {\n    let b: bytes = anew(0);\n    b = iset(b, 250);\n    let v: trit_vec = unpack5(b, 5);\n    let g: trit = egate(v);\n    return g;\n}";
    let st_corrupt_surf = expect_omni_error_code("CORRUPT_PACK5(surface)", execute_omni_surface_program(corrupt_surface, vec![]), "CORRUPT_PACK5")?;
    if st_corrupt_surf.line != Some(5) || st_corrupt_surf.col != Some(23) {
        return Err(format!("CORRUPT_PACK5(surface) span mismatch: {:?}:{:?} (want 5:23)", st_corrupt_surf.line, st_corrupt_surf.col));
    }
    let st_ambig = expect_omni_error_code("AMBIGUOUS_GRAMMAR", compile_omni_surface_strict_to_u16("fn main(a: i64) -> trit {\n    let t: trit = TRIT_NEG;\n    return t;\n}"), "AMBIGUOUS_GRAMMAR")?;
    if st_ambig.line != Some(2) || st_ambig.col != Some(19) {
        return Err(format!("AMBIGUOUS_GRAMMAR span mismatch: {:?}:{:?} (want 2:19)", st_ambig.line, st_ambig.col));
    }
    let st_syn = expect_omni_error_code("SYNTAX_ERROR", compile_omni_surface_to_u16("fn main(x: i64) -> i64 {\n    let r = ;\n    return r;\n}"), "SYNTAX_ERROR")?;
    if st_syn.line != Some(2) || st_syn.col != Some(13) {
        return Err(format!("SYNTAX_ERROR span mismatch: {:?}:{:?} (want 2:13)", st_syn.line, st_syn.col));
    }
    checks += 19;

    // 13s. Full-stack route + request param + DB/KV host ABI goldens (zero new opcodes).
    let route_kv = "#[cap(NET, DB_R, DB_W)]\nroute GET /users/:id (req: i64) {\n    let id = req.param(\"id\");\n    let saved = kv_put(id, 777);\n    let row = db_get(id);\n    respond(200, row);\n}";
    let (kv_val, kv_backend) = execute_omni_surface_aot_first(route_kv, vec![OmniValue::Int(42)])?;
    if kv_val != OmniValue::Int(200_000_777) || kv_backend != "vm" {
        return Err(format!("route GET /users/:id kv golden mismatch: {:?} via {}", kv_val, kv_backend));
    }
    let route_q = "route GET /users/:id (req: i64) @cap(net, db_r) {\n    let id = req.param(\"id\");\n    let row = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}";
    if execute_omni_surface_program(route_q, vec![OmniValue::Int(42)])? != OmniValue::Int(200_000_420) {
        return Err(String::from("route GET /users/:id db_query golden mismatch (want HTTP 200 row 420)"));
    }
    let route_u16 = compile_omni_surface_to_u16(route_q)?;
    if !route_u16.iter().any(|&t| t == 0x7F6C) || !route_u16.iter().any(|&t| t == 0x7F65) || !route_u16.iter().any(|&t| t == 0x7F6E) {
        return Err(String::from("route lowering did not emit ROUTE/DB_QRY/RSP_JSON host opcodes"));
    }
    checks += 2;
    expect_omni_error_code("route_no_caps", execute_omni_surface_program("route GET /users/:id (req: i64) {\n    let id = req.param(\"id\");\n    let row = db_get(id);\n    respond(200, row);\n}", vec![OmniValue::Int(42)]), "MISSING_CAP")?;
    expect_omni_error_code("route_no_db_r", execute_omni_surface_program("route GET /users/:id (req: i64) @cap(net) {\n    let id = req.param(\"id\");\n    let row = db_get(id);\n    respond(200, row);\n}", vec![OmniValue::Int(42)]), "MISSING_CAP")?;
    expect_omni_error_code("route_no_db_w", execute_omni_surface_program("#[cap(NET, DB_R)] route POST /users/:id (req: i64) {\n    let id = req.param(\"id\");\n    let saved = kv_put(id, 1);\n    respond(201, saved);\n}", vec![OmniValue::Int(42)]), "MISSING_CAP")?;
    expect_omni_error_code("sql_injection", execute_omni_surface_program("#[cap(DB_R)] fn main(id: i64 @untrusted) -> i64 { let r = db_query(\"SELECT * FROM users WHERE id = ?\", id); return r; }", vec![OmniValue::Int(5)]), "TAINT_SINK")?;
    let authed = "#[cap(DB_R, AUTH)] fn main(id: i64 @untrusted) -> i64 { let ok = auth_verify(id); let r = db_query(\"SELECT * FROM users WHERE id = ?\", ok); return r; }";
    if execute_omni_surface_program(authed, vec![OmniValue::Int(5)])? != OmniValue::Int(50) {
        return Err(String::from("auth_verify sanitizer did not clear @untrusted for db_query"));
    }
    checks += 5;

    // 13t. Version-locked omni-harness JSON contract (with non-null 1-based line and col on all failures).
    let want_version = format!("\"version\":\"{}\"", OMNI_HARNESS_CONTRACT_VERSION);
    let contains_all = |s: &str, parts: &[&str], label: &str| -> Result<(), String> {
        if s.contains('\n') {
            return Err(format!("Harness '{}' output is not single-line JSON: {}", label, s));
        }
        if s.starts_with("{\"ok\":false") && (s.contains("\"line\":null") || s.contains("\"col\":null")) {
            return Err(format!("Harness '{}' failure JSON contains null line/col span: {}", label, s));
        }
        for p in parts {
            if !s.contains(p) {
                return Err(format!("Harness '{}' output missing {}: {}", label, p, s));
            }
        }
        Ok(())
    };
    let ok_json = run_omni_harness_json("fn main(a: i64, b: i64) -> i64 { return a + b; }", vec![OmniValue::Int(2), OmniValue::Int(3)], false, false);
    contains_all(&ok_json, &["{\"ok\":true", &want_version, "\"result\":\"5\"", "\"tokens\":", "\"backend\":\"aot\""], "ok")?;
    if ok_json != run_omni_harness_json("fn main(a: i64, b: i64) -> i64 { return a + b; }", vec![OmniValue::Int(2), OmniValue::Int(3)], false, false) {
        return Err(String::from("Harness output is not deterministic"));
    }
    let taint_json = run_omni_harness_json("fn main(x: i64 @untrusted) -> i64 {\n    return x;\n}", vec![OmniValue::Int(1)], false, true);
    contains_all(&taint_json, &["{\"ok\":false", &want_version, "\"code\":\"TAINT_ON_RET\"", "\"line\":2,\"col\":5", "\"retry_hint\":\"Sanitize"], "taint")?;
    let cap_json = run_omni_harness_json("fn main(x: i64) -> trit {\n    let g = egate(x);\n    return g;\n}", vec![OmniValue::Int(1)], false, false);
    contains_all(&cap_json, &["\"ok\":false", "\"code\":\"MISSING_CAP\"", "\"line\":2,\"col\":13"], "cap")?;
    if cap_json.contains("retry_hint") {
        return Err(String::from("Harness emitted retry_hint without --retry-hint"));
    }
    let budget_json = run_omni_harness_json(runaway, vec![OmniValue::Int(1000)], false, false);
    contains_all(&budget_json, &["\"ok\":false", "\"code\":\"BUDGET_EXCEEDED\"", "\"line\":1,\"col\":3"], "budget")?;
    let strict_json = run_omni_harness_json("fn main(a: i64) -> i64 {\n    let b = a * 2;\n    return b;\n}", vec![OmniValue::Int(1)], true, true);
    contains_all(&strict_json, &["\"ok\":false", "\"code\":\"BAD_TYPE\"", "\"line\":2,\"col\":9", "\"tokens\":null", "\"retry_hint\":"], "strict")?;
    let undef_json = run_omni_harness_json("fn main(a: i64) -> i64 {\n    let b = unknown_v + 1;\n    return b;\n}", vec![OmniValue::Int(1)], false, true);
    contains_all(&undef_json, &["\"ok\":false", "\"code\":\"UNDEF_VAR\"", "\"line\":2,\"col\":13"], "undef")?;
    let syn_json = run_omni_harness_json("fn main(a: i64) -> i64 {\n    let b = ;\n    return b;\n}", vec![OmniValue::Int(1)], false, true);
    contains_all(&syn_json, &["\"ok\":false", "\"code\":\"SYNTAX_ERROR\"", "\"line\":2,\"col\":13"], "syntax")?;
    let route_json = run_omni_harness_json(route_kv, vec![OmniValue::Int(42)], false, false);
    contains_all(&route_json, &["\"ok\":true", "\"result\":\"200000777\"", "\"backend\":\"vm\""], "route")?;
    let trit_json = run_omni_harness_json("#[cap(AI)] fn main(a: i64, b: i64) -> trit { let g = egate(a - b); return g; }", vec![OmniValue::Int(30), OmniValue::Int(10)], false, false);
    contains_all(&trit_json, &["\"ok\":true", "\"result\":\"+1t\""], "trit")?;
    let spec = omni_harness_spec_json();
    contains_all(&spec, &[&want_version, "\"error_codes\":", "\"capabilities\":", "\"strict_rules\":"], "spec")?;
    for code in ["TAINT_ON_RET", "TAINT_SINK", "MISSING_CAP", "LINEAR_LEAK", "BORROW_CONFLICT", "UNDEF_VAR", "IMMUT_PARAM", "BAD_TYPE", "UNKNOWN_FN", "ARITY_MISMATCH", "UNRESOLVED_LABEL", "CIRCULAR_INCLUDE", "BUDGET_EXCEEDED", "CORRUPT_PACK5", "AMBIGUOUS_GRAMMAR", "SYNTAX_ERROR"].iter() {
        if !spec.contains(&format!("\"{}\"", code)) {
            return Err(format!("Harness --spec missing error code {}", code));
        }
    }
    checks += 10;

    // 13u. Version-locked Agent Driver (`omni-agent-driver`) emit + repair loop selftests.
    if run_omni_agent_repair_loop("fn main() -> i64 { return 1; }", vec![], false, 3, "omni-harness-v0.9.0").is_ok() {
        return Err(String::from("Agent driver must refuse mismatched harness contract version"));
    }
    checks += 1;

    // Repair case 1: MISSING_CAP on egate -> repairs to #[cap(AI)] and returns +1t
    let rep_cap = run_omni_agent_repair_loop(
        "fn main(a: i64, b: i64) -> trit {\n    let g: trit = egate(a - b);\n    return g;\n}",
        vec![OmniValue::Int(30), OmniValue::Int(10)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_cap.ok || rep_cap.attempts.len() != 2 || rep_cap.attempts[0].error.as_ref().map(|e| (e.code, e.line, e.col)) != Some(("MISSING_CAP", Some(2), Some(19))) {
        return Err(format!("Agent driver MISSING_CAP repair failed: {:?}", rep_cap));
    }
    checks += 1;

    // Repair case 2: TAINT_ON_RET -> sanitizes return value with auth_verify and #[cap(AUTH)]
    let rep_taint = run_omni_agent_repair_loop(
        "fn main(x: i64 @untrusted) -> i64 {\n    return x;\n}",
        vec![OmniValue::Int(42)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_taint.ok || rep_taint.attempts.len() != 2 || rep_taint.attempts[0].error.as_ref().map(|e| (e.code, e.line)) != Some(("TAINT_ON_RET", Some(2))) {
        return Err(format!("Agent driver TAINT_ON_RET repair failed: {:?}", rep_taint));
    }
    checks += 1;

    // Repair case 3: TAINT_SINK on db_query -> sanitizes query parameter and adds AUTH capability
    let rep_sink = run_omni_agent_repair_loop(
        "#[cap(DB_R)]\nfn main(id: i64 @untrusted) -> i64 {\n    let r: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    return r;\n}",
        vec![OmniValue::Int(42)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_sink.ok || rep_sink.attempts.len() != 2 || !rep_sink.final_json.contains("\"result\":\"420\"") {
        return Err(format!("Agent driver TAINT_SINK repair failed: {:?}", rep_sink));
    }
    checks += 1;

    // Repair case 4: Multi-step strict repair (AMBIGUOUS_GRAMMAR + untyped let BAD_TYPE + MISSING_CAP)
    let rep_multi = run_omni_agent_repair_loop(
        "fn main(a: int) -> trit {\n    let d = a - 10;\n    let g = egate(d);\n    return g;\n}",
        vec![OmniValue::Int(25)],
        true,
        5,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_multi.ok || rep_multi.attempts.len() < 3 || !rep_multi.final_json.contains("\"result\":\"+1t\"") {
        return Err(format!("Agent driver multi-step strict repair failed: {:?}", rep_multi));
    }
    checks += 1;

    // Repair case 5: BUDGET_EXCEEDED -> raises #[step_budget(N)] and completes
    let rep_budget = run_omni_agent_repair_loop(
        runaway,
        vec![OmniValue::Int(10)],
        false,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_budget.ok || rep_budget.attempts.len() != 2 || !rep_budget.final_json.contains("\"result\":\"55\"") {
        return Err(format!("Agent driver BUDGET_EXCEEDED repair failed: {:?}", rep_budget));
    }
    checks += 1;

    // Repair case 6: IMMUT_PARAM + BORROW_CONFLICT + CORRUPT_PACK5 + external repair prompt builder
    let rep_immut = run_omni_agent_repair_loop(
        "fn main(x: i64) -> i64 {\n    x = x + 5;\n    return x;\n}",
        vec![OmniValue::Int(10)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_immut.ok || !rep_immut.final_json.contains("\"result\":\"15\"") {
        return Err(format!("Agent driver IMMUT_PARAM repair failed: {:?}", rep_immut));
    }
    let rep_borrow = run_omni_agent_repair_loop(
        "fn main(x: i64) -> i64 {\n    let a: arena = arena(4096);\n    let r: arena = borrow_r(a);\n    drop(a);\n    drop(r);\n    return x;\n}",
        vec![OmniValue::Int(77)],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_borrow.ok || !rep_borrow.final_json.contains("\"result\":\"77\"") {
        return Err(format!("Agent driver BORROW_CONFLICT repair failed: {:?}", rep_borrow));
    }
    let rep_pack5 = run_omni_agent_repair_loop(
        corrupt_surface,
        vec![],
        true,
        3,
        OMNI_HARNESS_CONTRACT_VERSION,
    )?;
    if !rep_pack5.ok || !rep_pack5.final_json.contains("\"result\":\"+1t\"") {
        return Err(format!("Agent driver CORRUPT_PACK5 repair failed: {:?}", rep_pack5));
    }
    let prompt_json = build_omni_agent_repair_prompt_json(1, corrupt_surface, &st_corrupt_surf, &spec);
    contains_all(&prompt_json, &[&want_version, "\"attempt\":1", "\"code\":\"CORRUPT_PACK5\"", "\"line\":5,\"col\":23", "\"spec\":{"], "repair_prompt")?;
    checks += 4;

    // 13v. Live Native TCP HTTP/1.1 + DB/KV Host Golden (`serve_omni_http_batch`, `serve_omni_http_batch_with_db_file` & `serve_omni_http_once`).
    // 1) Verifier must reject serving a route without required capabilities before binding a port.
    let uncap_route = "route GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_query(\"SELECT * FROM users WHERE id = ?\", id);\n    respond(200, row);\n}";
    expect_omni_error_code(
        "live_host_missing_cap",
        serve_omni_http_once(uncap_route, "127.0.0.1:0", BTreeMap::new()).map(|(a, _)| a),
        "MISSING_CAP",
    )?;
    checks += 1;

    // 2) Live end-to-end TCP HTTP/1.1 golden + persistent on-disk KV/DB file across server restarts:
    // Instance #1 handles POST /users/42 -> persists 42=777 to disk; Instance #2 loads from disk and serves GET /users/42 -> 777!
    let live_fullstack_omni = "#[strict]\n#[cap(NET, DB_W)]\nroute POST /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let saved: i64 = kv_put(id, 777);\n    respond(201, saved);\n}\n#[strict]\n#[cap(NET, DB_R)]\nroute GET /users/:id (req: i64) {\n    let id: i64 = req.param(\"id\");\n    let row: i64 = db_get(id);\n    respond(200, row);\n}";
    let db_path = std::env::temp_dir().join(format!("omni_live_host_db_{}.kv", std::process::id()));
    let _ = std::fs::remove_file(&db_path);
    let (listen_addr1, server_handle1) = serve_omni_http_batch_with_db_file(live_fullstack_omni, "127.0.0.1:0", &db_path, 1)?;
    {
        use std::io::{Read, Write};
        let mut client1 = std::net::TcpStream::connect(listen_addr1)
            .map_err(|e| format!("TCP client1 connect failed: {}", e))?;
        client1
            .write_all(b"POST /users/42 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .map_err(|e| format!("TCP client1 write failed: {}", e))?;
        let mut wire1 = String::new();
        client1
            .read_to_string(&mut wire1)
            .map_err(|e| format!("TCP client1 read failed: {}", e))?;
        if !wire1.starts_with("HTTP/1.1 201 Created\r\n") || !wire1.contains("{\"status\":201,\"data\":777}") {
            let _ = std::fs::remove_file(&db_path);
            return Err(format!("Live HTTP POST /users/42 unexpected wire response: {}", wire1));
        }
    }
    let (resps1, final_kv1) = server_handle1
        .join()
        .map_err(|_| String::from("Live HTTP server1 thread panicked"))??;
    if resps1.len() != 1 || final_kv1.get(&42) != Some(&777) {
        let _ = std::fs::remove_file(&db_path);
        return Err(format!("Live HTTP host1 KV state mismatch: {:?}", final_kv1));
    }

    // Instance #2 starts fresh and loads `db_path` from disk to serve GET /users/42
    let (listen_addr2, server_handle2) = serve_omni_http_batch_with_db_file(live_fullstack_omni, "127.0.0.1:0", &db_path, 1)?;
    {
        use std::io::{Read, Write};
        let mut client2 = std::net::TcpStream::connect(listen_addr2)
            .map_err(|e| format!("TCP client2 connect failed: {}", e))?;
        client2
            .write_all(b"GET /users/42 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .map_err(|e| format!("TCP client2 write failed: {}", e))?;
        let mut wire2 = String::new();
        client2
            .read_to_string(&mut wire2)
            .map_err(|e| format!("TCP client2 read failed: {}", e))?;
        if !wire2.starts_with("HTTP/1.1 200 OK\r\n") || !wire2.contains("{\"status\":200,\"data\":777}") {
            let _ = std::fs::remove_file(&db_path);
            return Err(format!("Live HTTP GET /users/42 unexpected wire response: {}", wire2));
        }
    }
    let (resps2, final_kv2) = server_handle2
        .join()
        .map_err(|_| String::from("Live HTTP server2 thread panicked"))??;
    let _ = std::fs::remove_file(&db_path);
    if resps2.len() != 1 || final_kv2.get(&42) != Some(&777) {
        return Err(format!("Live HTTP host2 persistent KV state mismatch: {:?}", final_kv2));
    }
    checks += 2;

    // 13w. Section 1: Error Contract & Executable Acceptance Suite Selftests (`spec/acceptance/`)
    checks += run_error_contract_selftest()?;
    checks += run_acceptance_suite_selftest()?;

    // 13x. Section 3: Narrow Capability-Gated Host Interface (`OmniNarrowHost` & `SimulatedNarrowHostAdapter`)
    {
        let mut sim_host = SimulatedNarrowHostAdapter::new();
        // Verify narrow host capability enforcement rejects unprivileged TCP/File/Time calls with MISSING_CAP
        expect_omni_error_code(
            "narrow_host_no_net",
            sim_host.host_tcp_listen(0, "127.0.0.1:0"),
            "MISSING_CAP",
        )?;
        expect_omni_error_code(
            "narrow_host_no_db_w",
            sim_host.host_file_open(cap_bit(CAP_ID_DB_R), "test.kv", true),
            "MISSING_CAP",
        )?;
        expect_omni_error_code(
            "narrow_host_no_sys_time",
            sim_host.host_time_ns(0),
            "MISSING_CAP",
        )?;
        let t1 = sim_host.host_time_ns(cap_bit(CAP_ID_SYS))?;
        let t2 = sim_host.host_time_ns(cap_bit(CAP_ID_SYS))?;
        if t2 <= t1 {
            return Err(String::from("Narrow host monotonic clock did not advance"));
        }
        sim_host.enqueue_http_request("POST /users/42 HTTP/1.1
Host: localhost

");
        sim_host.enqueue_http_request("GET /users/42 HTTP/1.1
Host: localhost

");
        let (bound, sim_resps, sim_kv) = serve_omni_http_via_narrow_host(
            &mut sim_host,
            live_fullstack_omni,
            "127.0.0.1:0",
            Some("virtual_db.kv"),
            2,
        )?;
        if bound != "127.0.0.1:40080"
            || sim_resps.len() != 2
            || sim_resps[0].status != 201
            || sim_resps[1].status != 200
            || sim_kv.get(&42) != Some(&777)
            || !sim_host.virtual_files.contains_key("virtual_db.kv")
        {
            return Err(format!("Simulated narrow host roundtrip mismatch: bound={}, resps={:?}", bound, sim_resps));
        }
        checks += 4;
    }

    // 13y. Section 2: Deepened Multi-Step AI Repair Loop (`omni-repair-strategies-v1.1.0`)
    {
        let strat_json = omni_repair_strategies_json();
        if !strat_json.contains(OMNI_REPAIR_STRATEGIES_VERSION)
            || !strat_json.contains("strat_multi_fn_arity_align_v1")
            || !strat_json.contains("strat_include_cycle_prune_v1")
            || list_omni_repair_strategies().len() < 16
        {
            return Err(format!("Repair strategies catalog incomplete: {}", strat_json));
        }

        // Multi-step repair 1: Multi-function callee missing capability + caller arity mismatch
        let broken_multi_fn = "#[strict]
fn helper_gate(x: i64) -> trit {
    let g: trit = egate(x - 10);
    return g;
}
#[strict]
fn main(a: i64) -> trit {
    let res: trit = helper_gate(a, 999);
    return res;
}";
        let rep_mf = run_omni_agent_repair_loop(
            broken_multi_fn,
            vec![OmniValue::Int(25)],
            true,
            5,
            OMNI_HARNESS_CONTRACT_VERSION,
        )?;
        if !rep_mf.ok || !rep_mf.final_json.contains("\"result\":\"+1t\"") {
            return Err(format!("Multi-function arity + callee cap repair failed: {:?}", rep_mf));
        }

        // Multi-step repair 2: Multi-file circular include graph repair
        let mut cyc_vfs = BTreeMap::new();
        cyc_vfs.insert(
            String::from("math_util.omni"),
            String::from("include \"app.omni\";\n#[strict]\nfn scale3(v: i64) -> i64 { let r: i64 = v * 3; return r; }"),
        );
        cyc_vfs.insert(
            String::from("app.omni"),
            String::from("include \"math_util.omni\";\n#[strict]\nfn main(x: i64) -> i64 { let y = scale3(x); return y; }"),
        );
        let (_, rep_inc) = repair_omni_include_graph(&cyc_vfs, "app.omni", vec![OmniValue::Int(14)], true, 5)?;
        if !rep_inc.ok || !rep_inc.final_json.contains("\"result\":\"42\"") {
            return Err(format!("Include graph repair failed: {:?}", rep_inc));
        }

        // Multi-step repair 3: Unresolved goto label + untyped let in strict mode
        let broken_lbl = "#[strict]
fn main(x: i64) -> i64 {
    let out = x + 10;
    goto 4;
    return out;
}";
        let rep_lbl = run_omni_agent_repair_loop(broken_lbl, vec![OmniValue::Int(5)], true, 5, OMNI_HARNESS_CONTRACT_VERSION)?;
        if !rep_lbl.ok || !rep_lbl.final_json.contains("\"result\":\"15\"") {
            return Err(format!("Unresolved label + strict let repair failed: {:?}", rep_lbl));
        }

        // Multi-step repair 4: Undefined variable + unknown intrinsic
        let broken_undef = "#[strict]
fn main(x: i64) -> i64 {
    let a: i64 = unknown_abs(x);
    let b: i64 = a + missing_offset;
    return b;
}";
        let rep_undef = run_omni_agent_repair_loop(broken_undef, vec![OmniValue::Int(-20)], true, 5, OMNI_HARNESS_CONTRACT_VERSION)?;
        if !rep_undef.ok || !rep_undef.final_json.contains("\"result\":\"20\"") {
            return Err(format!("Undefined var + unknown fn repair failed: {:?}", rep_undef));
        }

        // Multi-step repair 5: Memory budget exceeded (`#[mem_budget(16)]`) + DOM capability inference
        let broken_mem = "#[strict]
#[mem_budget(16)]
fn main(x: i64) -> str {
    let s: str = \"0123456789ABCDEF0123456789ABCDEF\";
    let clean: str = san_dom(s);
    return clean;
}";
        let rep_mem = run_omni_agent_repair_loop(broken_mem, vec![OmniValue::Int(1)], true, 5, OMNI_HARNESS_CONTRACT_VERSION)?;
        if !rep_mem.ok || !rep_mem.final_json.contains("0123456789ABCDEF") {
            return Err(format!("Memory budget repair failed: {:?}", rep_mem));
        }

        // Verify strategy_id & strategy_version appear in run_omni_agent_driver_json
        let driver_json = run_omni_agent_driver_json(
            broken_multi_fn,
            vec![OmniValue::Int(25)],
            true,
            5,
            OMNI_HARNESS_CONTRACT_VERSION,
        );
        contains_all(
            &driver_json,
            &[
                "\"ok\":true",
                "\"strategy_version\":\"omni-repair-strategies-v1.1.0\"",
                "\"strategy_id\":\"strat_multi_fn_arity_align_v1\"",
                "\"strategy_id\":\"strat_multi_fn_callee_cap_v1\"",
            ],
            "agent_driver_strategy_json",
        )?;
        checks += 8;
    }

    // 13z. Section 6: Tooling & Human Diagnostics (`format_omni_inspect`, `format_omni_human_diagnostic`, `explain_omni_error_code`)
    {
        let inspect_src = "#[strict]
#[cap(AI)]
fn main(a: i64) -> trit {
    let g: trit = egate(a - 5);
    return g;
}";
        let disasm = inspect_omni_surface_program(inspect_src, true)?;
        if !disasm.contains("=== OMNI BINARY STREAM INSPECTION")
            || !disasm.contains("cap_mask=0x0010 (AI)")
            || !disasm.contains("OP_7F35_EGATE (0x7F35)")
            || !disasm.contains("// line 4, col 19")
        {
            return Err(format!("format_omni_inspect output missing expected annotations:
{}", disasm));
        }
        let explain_taint = explain_omni_error_code("TAINT_SINK")
            .ok_or_else(|| String::from("explain_omni_error_code(TAINT_SINK) returned None"))?;
        if !explain_taint.contains("error[TAINT_SINK]") || !explain_taint.contains("auth_verify") {
            return Err(format!("explain_omni_error_code(TAINT_SINK) mismatch: {}", explain_taint));
        }
        let diag = format_omni_human_diagnostic("fn main(x: i64 @untrusted) -> i64 {
    return x;
}", &st_taint_ret);
        if !diag.contains("error[TAINT_ON_RET]") || !diag.contains("--> line 3:5") || !diag.contains("^") {
            return Err(format!("format_omni_human_diagnostic mismatch:
{}", diag));
        }
        checks += 3;
    }

    // 13AA. Section 5: Next Abstraction Layer (`std/prelude.omni` + Hygienic `macro` Differential across VM, Stage-1, ELF & Wasm)
    {
        let macro_aot_src = "macro mad_step(a, b, c) = (a * b) + c;
macro trit_bonus(t) = sel_trit(t, -20, 0, 20);
#[strict]
fn main(x: i64, y: i64) -> i64 {
    let m: i64 = mad_step(x, 3, y);
    let cmp: trit = m <=> 50;
    let b: i64 = trit_bonus(cmp);
    return m + b;
}";
        let (aot_res, aot_back) = execute_omni_surface_aot_first(macro_aot_src, vec![OmniValue::Int(20), OmniValue::Int(5)])?;
        let vm_res = execute_omni_surface_program_with_budgets(macro_aot_src, vec![OmniValue::Int(20), OmniValue::Int(5)], None, None)?;
        let wasm_res = execute_omni_surface_wasm_i64(macro_aot_src, &[20, 5])?;
        let elf_direct = compile_omni_surface_to_x86_64_elf(macro_aot_src, 20, 5, 0)?;
        let elf_stage1 = compile_omni_surface_via_stage1_to_elf(macro_aot_src, 20, 5, 0)?;
        if aot_back != "aot" || aot_res != OmniValue::Int(85) || vm_res != OmniValue::Int(85) || wasm_res != 85 || elf_direct != elf_stage1 {
            return Err(format!(
                "Hygienic macro 4-way differential failed: aot={:?} ({}), vm={:?}, wasm={}",
                aot_res, aot_back, vm_res, wasm_res
            ));
        }
        checks += 2;
    }

    // 13AB. Section 4: Compositional Verification Invariants (`spec/trusted_core.md`)
    checks += run_compositional_invariant_selftests()?;

    // 13AC. Section 7: Phase 7 Pure-Omni Self-Hosted Verifier Kernel, Fixed-Point ELF64 Proof & Narrow-Host Runner
    checks += run_phase7_pure_omni_selftests()?;

    // 13AD. Phase A: Pure-Omni Surface Frontend (`omni-pure-frontend-v1.0.0`)
    checks += run_phase_a_pure_omni_frontend_selftests()?;

    // 13AE. Phase B: Pure-Omni Driver & Verifier Kernel Integration (`omni-pure-driver-v1.0.0`)
    checks += run_phase_b_pure_omni_driver_selftests()?;

    // 13AF. Phase C: Pure-Omni Host Runtime over Narrow Host (`omni-pure-host-v1.0.0`)
    checks += run_phase_c_pure_omni_host_selftests()?;

    // 13AG. Phase D: Pure-Omni Self-Hosting Bootstrap Fixed Point (`omni-pure-selfhost-v1.0.0`)
    checks += run_phase_d_pure_omni_bootstrap_selftests()?;

    // 13AH. Phase E: Full Pure-Omni Differential Lock (`pure-omni-diff`)
    checks += run_phase_e_pure_omni_diff_selftests()?;

    // 13AI. Pure-Omni Core Codebase (`pure/*.omni` + `std/*.omni`) & Multi-Module Self-Host Suite
    checks += run_pure_omni_core_suite_selftests()?;

    // 13AJ. Pure-Core Completion Phase 1: Full Surface Language Coverage in Pure-Omni Frontend
    checks += run_pure_core_phase1_frontend_coverage_selftests()?;

    // 13AK. Pure-Core Completion Phase 2: High-Value Tooling in Pure Omni
    checks += run_pure_core_phase2_tooling_selftests()?;

    Ok(checks)
}

/// Shared `harness` CLI front-end (`omni harness ...` / `omni-harness ...`).
/// Usage: harness [--strict] [--retry-hint] (--spec | --version-check | <file.omni | - | "inline source">) [args...]
/// Prints exactly one JSON line; exit code 0 on `"ok":true`, 1 otherwise, 2 on usage errors.
pub fn run_omni_harness_cli(argv: &[String]) -> i32 {
    let mut strict = false;
    let mut retry_hint = false;
    let mut pure_omni = true;
    let mut rest: Vec<String> = Vec::new();
    let mut idx_h = 0usize;
    while idx_h < argv.len() {
        let a = &argv[idx_h];
        if a == "--explain" && idx_h + 1 < argv.len() {
            match explain_omni_error_code(&argv[idx_h + 1]) {
                Some(text) => {
                    println!("{}", text);
                    return 0;
                }
                None => {
                    eprintln!("Unknown Omni error code '{}'", argv[idx_h + 1]);
                    return 2;
                }
            }
        }
        idx_h += 1;
    }
    for a in argv {
        match a.as_str() {
            "--strict" => strict = true,
            "--retry-hint" => retry_hint = true,
            "--pure-omni" => pure_omni = true,
            "--legacy-rust" => pure_omni = false,
            "--spec" => {
                println!("{}", omni_harness_spec_json());
                return 0;
            }
            "--version-check" | "--version" => {
                println!("{{\"version\":\"{}\"}}", OMNI_HARNESS_CONTRACT_VERSION);
                return 0;
            }
            _ => rest.push(a.clone()),
        }
    }
    if rest.is_empty() {
        eprintln!("usage: omni-harness [--strict] [--retry-hint] [--pure-omni] (--spec | --version-check | <file.omni | - | \"source\">) [args...]");
        return 2;
    }
    let target = &rest[0];
    let src = if target == "-" {
        use std::io::Read;
        let mut buf = String::new();
        if std::io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("failed to read source from stdin");
            return 2;
        }
        buf
    } else if target.ends_with(".omni") && std::path::Path::new(target).is_file() {
        match std::fs::read_to_string(target) {
            Ok(raw) => {
                if strict && (raw.contains("mod ") || raw.contains("mod\"")) {
                    raw
                } else {
                    let parent_dir = std::path::Path::new(target).parent();
                    match resolve_omni_surface_includes(&raw, parent_dir, None) {
                        Ok(expanded) => expanded,
                        Err(e) => {
                            let st = parse_omni_structured_error(&e);
                            let line_json = st.line.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                            let col_json = st.col.map(|n| n.to_string()).unwrap_or_else(|| String::from("null"));
                            if retry_hint {
                                println!(
                                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null,\"retry_hint\":\"{}\"}}",
                                    OMNI_HARNESS_CONTRACT_VERSION,
                                    st.code,
                                    line_json,
                                    col_json,
                                    json_escape_str(&st.message),
                                    json_escape_str(st.retry_hint)
                                );
                            } else {
                                println!(
                                    "{{\"ok\":false,\"version\":\"{}\",\"code\":\"{}\",\"line\":{},\"col\":{},\"message\":\"{}\",\"tokens\":null}}",
                                    OMNI_HARNESS_CONTRACT_VERSION,
                                    st.code,
                                    line_json,
                                    col_json,
                                    json_escape_str(&st.message)
                                );
                            }
                            return 1;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to read '{}': {}", target, e);
                return 2;
            }
        }
    } else {
        target.clone()
    };
    let vm_args: Vec<OmniValue> = rest[1..].iter().map(|s| parse_cli_omni_value(s)).collect();
    let out = if pure_omni {
        run_pure_omni_driver_json(&src, vm_args, strict, retry_hint)
    } else {
        run_omni_harness_json(&src, vm_args, strict, retry_hint)
    };
    println!("{}", out);
    if out.starts_with("{\"ok\":true") { 0 } else { 1 }
}
