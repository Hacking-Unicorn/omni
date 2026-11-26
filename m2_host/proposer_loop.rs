// ==============================================================================
// STEP 5: CLOSED-LOOP PROPOSER (`spec/m7/PROPOSER_LOOP_DESIGN.md`)
// ==============================================================================
// Untrusted proposer -> Immutable TCB (Stage 0..4) -> Pure-Omni Judge/Heal/Learn
// -> Step 4 Audit View -> Git-backed SHA-256 content-addressed store (`flywheel/auto`).
// Zero external dependencies. Never modifies verifier, judge, heal, or frozen hashes.

/// Pure-Rust FIPS 180-4 SHA-256 (zero external crates) for content-addressed candidate storage
/// and Stage-0 immutable deny-list digest locking.
pub fn proposer_sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{:08x}", word));
    }
    out
}

/// Immutable TCB Deny-List (`spec/m7/PROPOSER_LOOP_DESIGN.md` §5).
/// The proposer is mechanically prevented from modifying any path in this list.
pub const PROPOSER_IMMUTABLE_DENY_LIST: &[&str] = &[
    "pure/verifier_engine.omni",
    "spec/m2/single_mode_lock.tsv",
    "spec/m2/shape_census.tsv",
    "spec/m3/corpus.tsv",
    "omni_rust_bootstrap.rs",
    "scripts/external_verifier_check.py",
    "pure/m7/judge.omni",
    "pure/m7/heal.omni",
    "pure/isa/table.omni",
    "pure/m2/emit_x86.omni",
    "seed0/omni_seed0.rs",
    "p4_shim/mod.rs",
    "pure/m5/launcher.omni",
    "scripts/zero_rust_rebuild.sh",
    ".github/workflows/ci.yml",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposerTargetSubsystem {
    Tier1Tokens,
    M2Emitter,
    M3Frontend,
    M4Vm,
}

impl ProposerTargetSubsystem {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tier1Tokens => "TIER1_TOKENS",
            Self::M2Emitter => "M2_EMITTER",
            Self::M3Frontend => "M3_FRONTEND",
            Self::M4Vm => "M4_VM",
        }
    }

    pub fn attribution_id(&self) -> i64 {
        match self {
            Self::M3Frontend => 1,
            Self::M2Emitter => 2,
            Self::M4Vm => 3,
            Self::Tier1Tokens => 2, // Tier-1 token reductions lower ELF/VM metrics
        }
    }
}

/// Stage 0 TCB Scope & Integrity Guard.
#[derive(Clone, Debug)]
pub struct ProposerScopeGuard {
    pub deny_digests: BTreeMap<String, String>,
    pub max_token_words: usize,
    pub allowed_cap_mask: CapMask,
}

impl ProposerScopeGuard {
    /// Snapshots SHA-256 digests of all immutable deny-listed files on disk.
    pub fn snapshot(max_token_words: usize, allowed_cap_mask: CapMask) -> Result<Self, String> {
        let mut deny_digests = BTreeMap::new();
        for &path in PROPOSER_IMMUTABLE_DENY_LIST {
            if std::path::Path::new(path).is_file() {
                let raw = std::fs::read(path)
                    .map_err(|e| format!("Stage0 snapshot read '{}': {}", path, e))?;
                deny_digests.insert(String::from(path), proposer_sha256_hex(&raw));
            }
        }
        Ok(Self {
            deny_digests,
            max_token_words,
            allowed_cap_mask,
        })
    }

    /// Checks a list of candidate-touched paths against the single-subsystem allowlist and deny-list.
    /// Returns `Err((err_code, msg))` where `err_code == 10` is `VERIFIER_TAMPER` (`FATAL_ABORT`).
    pub fn check_paths(
        &self,
        target: ProposerTargetSubsystem,
        touched_paths: &[&str],
    ) -> Result<(), (i64, String)> {
        for &raw_path in touched_paths {
            let norm = raw_path.replace('\\', "/");
            // 1. Deny-list check (highest priority -> FATAL_ABORT err_code = 10)
            for &denied in PROPOSER_IMMUTABLE_DENY_LIST {
                if norm == denied
                    || norm.ends_with(&format!("/{}", denied))
                    || norm.starts_with("spec/verifier_traces/")
                    || norm.starts_with("oracle/")
                {
                    return Err((
                        10,
                        format!(
                            "VERIFIER_TAMPER: proposal attempted to modify immutable TCB path '{}'",
                            norm
                        ),
                    ));
                }
            }
            // 2. Single-subsystem allowlist check
            let allowed = match target {
                ProposerTargetSubsystem::Tier1Tokens => false, // Tier-1 must have 0 disk path edits
                ProposerTargetSubsystem::M2Emitter => norm == "pure/m2/emit_multi.omni",
                ProposerTargetSubsystem::M3Frontend => {
                    norm.starts_with("pure/m3/") && norm.ends_with(".omni")
                }
                ProposerTargetSubsystem::M4Vm => {
                    norm.starts_with("pure/m4/") && norm.ends_with(".omni")
                }
            };
            if !allowed {
                return Err((
                    2,
                    format!(
                        "SCOPE_VIOLATION: path '{}' is outside target subsystem {}",
                        norm,
                        target.as_str()
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Verifies post-proposal digests of all immutable deny-listed files match the pre-proposal snapshot.
    pub fn verify_deny_digests_intact(&self) -> Result<(), (i64, String)> {
        for (path, want_sha) in &self.deny_digests {
            let raw = std::fs::read(path).map_err(|e| {
                (
                    10,
                    format!("VERIFIER_TAMPER: deny-listed file '{}' unreadable: {}", path, e),
                )
            })?;
            let got_sha = proposer_sha256_hex(&raw);
            if &got_sha != want_sha {
                return Err((
                    10,
                    format!(
                        "VERIFIER_TAMPER: immutable file '{}' digest drifted ({} != {})",
                        path, got_sha, want_sha
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Enforces token-stream size cap and capability budget.
    pub fn check_token_budget(&self, stream: &[u16]) -> Result<(), (i64, String)> {
        if stream.len() > self.max_token_words {
            return Err((
                3,
                format!(
                    "SIZE_CAP_EXCEEDED: candidate {} words > cap {}",
                    stream.len(),
                    self.max_token_words
                ),
            ));
        }
        audit_validate_framing(stream).map_err(|e| (4, e))?;
        let m = decode_omni_token_stream(stream).map_err(|e| (4, e))?;
        for f in &m.functions {
            if (f.cap_mask & !self.allowed_cap_mask) != 0 {
                return Err((
                    5,
                    format!(
                        "CAP_BUDGET_EXCEEDED: fn_{} cap_mask=0x{:04X} exceeds allowed=0x{:04X}",
                        f.fn_idx, f.cap_mask, self.allowed_cap_mask
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// CEGIS + CEGIS(T) verifier gradient returned on rejection so the proposer can hill-climb.
#[derive(Clone, Debug)]
pub struct ProposerGradient {
    pub err_code: i64,
    pub assertion_violated: String,
    pub minimal_failing_input: Option<[i64; 3]>,
    pub expected_output: Option<i64>,
    pub actual_output: Option<i64>,
    pub localized_fn_idx: Option<u16>,
    pub localized_word_offset: Option<usize>,
    pub ranked_repair_templates: Vec<&'static str>,
}

/// Verdict returned by Stage 4 (`pure/m7/judge.omni::p7_adopt_rule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposerVerdict {
    Adopt,  // +1t
    Hold,   //  0t
    Reject, // -1t
}

/// Complete audit record of a single proposer candidate evaluation.
#[derive(Clone, Debug)]
pub struct ProposerCandidateOutcome {
    pub attempt: usize,
    pub candidate_sha256: String,
    pub target: ProposerTargetSubsystem,
    pub verdict: ProposerVerdict,
    pub baseline_words: usize,
    pub candidate_words: usize,
    pub baseline_elf_bytes: usize,
    pub candidate_elf_bytes: usize,
    pub baseline_vm_steps: i64,
    pub candidate_vm_steps: i64,
    pub gradient: Option<ProposerGradient>,
    pub audit_decompile: Option<String>,
    pub audit_diff_summary: Option<String>,
    pub git_commit_sha: Option<String>,
}

/// Git-backed content-addressed candidate store (`flywheel/auto` + per-agent isolation + sleep-time GC).
pub struct ProposerGitStore {
    pub root: std::path::PathBuf,
}

impl ProposerGitStore {
    /// Initializes an isolated Git-backed content-addressed store with a `flywheel/auto` branch.
    pub fn init(root: &std::path::Path) -> Result<Self, String> {
        let _ = std::fs::remove_dir_all(root);
        std::fs::create_dir_all(root.join("candidates"))
            .map_err(|e| format!("ProposerGitStore mkdir: {}", e))?;
        std::fs::create_dir_all(root.join("scratch"))
            .map_err(|e| format!("ProposerGitStore mkdir scratch: {}", e))?;
        let run_git = |args: &[&str]| -> Result<String, String> {
            let out = std::process::Command::new("git")
                .current_dir(root)
                .args(args)
                .output()
                .map_err(|e| format!("git {:?} spawn failed: {}", args, e))?;
            if !out.status.success() {
                return Err(format!(
                    "git {:?} failed: {}",
                    args,
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        };
        run_git(&["init", "-b", "flywheel/auto"])?;
        run_git(&["config", "user.name", "omni-proposer-tcb"])?;
        run_git(&["config", "user.email", "proposer@omni.local"])?;
        std::fs::write(
            root.join("README.md"),
            "# Omni Proposer Content-Addressed Store (flywheel/auto)\n",
        )
        .map_err(|e| e.to_string())?;
        run_git(&["add", "README.md"])?;
        run_git(&["commit", "-m", "init: flywheel/auto store"])?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Writes an unadopted candidate to `scratch/<sha256>.omni_bc` (eligible for sleep-time GC).
    pub fn record_scratch(&self, sha256: &str, stream: &[u16]) -> Result<std::path::PathBuf, String> {
        let p = self.root.join("scratch").join(format!("{}.omni_bc", sha256));
        std::fs::write(&p, encode_u16_stream_to_bytes(stream))
            .map_err(|e| format!("store write scratch: {}", e))?;
        Ok(p)
    }

    /// Auto-commits an adopted (`+1t`) candidate and its Step 4 audit view to `flywheel/auto`
    /// via an isolated agent branch `agent/<agent_id>` (never `main`).
    pub fn adopt_commit(
        &self,
        agent_id: &str,
        sha256: &str,
        stream: &[u16],
        audit_decompile: &str,
        audit_diff_summary: &str,
    ) -> Result<String, String> {
        let run_git = |args: &[&str]| -> Result<String, String> {
            let out = std::process::Command::new("git")
                .current_dir(&self.root)
                .args(args)
                .output()
                .map_err(|e| format!("git {:?} spawn failed: {}", args, e))?;
            if !out.status.success() {
                return Err(format!(
                    "git {:?} failed: {}",
                    args,
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        };
        let agent_branch = format!("agent/{}", agent_id);
        run_git(&["checkout", "-B", &agent_branch, "flywheel/auto"])?;
        let bc_rel = format!("candidates/{}.omni_bc", sha256);
        let dec_rel = format!("candidates/{}.decompile.omni", sha256);
        let diff_rel = format!("candidates/{}.audit_diff.txt", sha256);
        std::fs::write(self.root.join(&bc_rel), encode_u16_stream_to_bytes(stream))
            .map_err(|e| e.to_string())?;
        std::fs::write(self.root.join(&dec_rel), audit_decompile).map_err(|e| e.to_string())?;
        std::fs::write(self.root.join(&diff_rel), audit_diff_summary).map_err(|e| e.to_string())?;
        run_git(&["add", &bc_rel, &dec_rel, &diff_rel])?;
        let msg = format!(
            "flywheel(auto): adopt +1t candidate {} via {}",
            &sha256[..12],
            agent_id
        );
        run_git(&["commit", "-m", &msg])?;
        run_git(&["checkout", "flywheel/auto"])?;
        run_git(&["merge", "--ff-only", &agent_branch])?;
        run_git(&["rev-parse", "--short", "HEAD"])
    }

    /// Sleep-time garbage collection: prunes unadopted scratch artifacts while keeping all
    /// adopted SHA-256 candidates committed on `flywheel/auto`.
    pub fn sleep_time_gc(&self) -> Result<usize, String> {
        let mut removed = 0usize;
        let scratch_dir = self.root.join("scratch");
        if scratch_dir.is_dir() {
            for entry in std::fs::read_dir(&scratch_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().is_file() {
                    std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }
}

/// Evaluates a single Tier-1 `.omni_bc` candidate through Stages 0–4:
///   - Stage 0: TCB scope guard + deny-list digest lock + size/cap budget
///   - Stage 1: `api_token_emit` + Pure-Omni `omni_verify_module` + Rust `verify_binary_function`
///              + M2-multi ELF64 lowering + M4 VM & native Linux x86-64 semantic equivalence
///   - Stage 2: Canary proof (`p7-canary` pre-checked once per episode)
///   - Stage 3: 5-run sequential consensus with 0-variance check on deterministic metrics
///   - Stage 4: Pure-Omni `pure/m7/judge.omni::p7_adopt_rule` (`+1t`/`0t`/`-1t`) + Step 4 Audit View
pub fn proposer_evaluate_tier1_candidate(
    guard: &ProposerScopeGuard,
    baseline_u16: &[u16],
    candidate_u16: &[u16],
    inputs: [i64; 3],
    expected_ret: i64,
    canary_ok: bool,
    attempt: usize,
) -> Result<ProposerCandidateOutcome, String> {
    let cand_bytes = encode_u16_stream_to_bytes(candidate_u16);
    let cand_sha = proposer_sha256_hex(&cand_bytes);
    let base_elf = compile_pure_u16_to_x86_64_elf_multi(baseline_u16, inputs[0], inputs[1], inputs[2])?;
    let base_mod = decode_omni_token_stream(baseline_u16)?;
    let mut base_vm = OmniBinaryVm::new();
    let base_entry = base_mod.functions.last().map(|f| f.fn_idx).unwrap_or(0);
    let _ = base_vm.execute_function(
        &base_mod,
        base_entry,
        inputs.iter().take(base_mod.functions.last().map(|f| f.params.len()).unwrap_or(0)).map(|&v| OmniValue::Int(v)).collect(),
    )?;
    let base_steps = base_vm.steps_executed as i64;

    let make_reject = |err_code: i64,
                       assertion: &str,
                       fail_in: Option<[i64; 3]>,
                       exp: Option<i64>,
                       act: Option<i64>,
                       fn_idx: Option<u16>,
                       word_off: Option<usize>,
                       repairs: Vec<&'static str>| {
        ProposerCandidateOutcome {
            attempt,
            candidate_sha256: cand_sha.clone(),
            target: ProposerTargetSubsystem::Tier1Tokens,
            verdict: ProposerVerdict::Reject,
            baseline_words: baseline_u16.len(),
            candidate_words: candidate_u16.len(),
            baseline_elf_bytes: base_elf.len(),
            candidate_elf_bytes: 0,
            baseline_vm_steps: base_steps,
            candidate_vm_steps: 0,
            gradient: Some(ProposerGradient {
                err_code,
                assertion_violated: String::from(assertion),
                minimal_failing_input: fail_in,
                expected_output: exp,
                actual_output: act,
                localized_fn_idx: fn_idx,
                localized_word_offset: word_off,
                ranked_repair_templates: repairs,
            }),
            audit_decompile: None,
            audit_diff_summary: None,
            git_commit_sha: None,
        }
    };

    // Stage 0: TCB scope & deny-list digest integrity
    if let Err((code, msg)) = guard.verify_deny_digests_intact() {
        return Ok(make_reject(
            code,
            &msg,
            None,
            None,
            None,
            None,
            None,
            vec!["abort_and_restore_immutable_tcb"],
        ));
    }
    if let Err((code, msg)) = guard.check_token_budget(candidate_u16) {
        return Ok(make_reject(
            code,
            &msg,
            None,
            None,
            None,
            None,
            None,
            vec!["shrink_candidate_tokens", "preserve_cap_mask"],
        ));
    }

    // Stage 2 gate check (ruler must be unbroken)
    if !canary_ok {
        return Ok(make_reject(
            11,
            "CANARY_DRIFT: p7-canary did not hold +1t/0t",
            None,
            None,
            None,
            None,
            None,
            vec!["halt_loop_and_inspect_canary"],
        ));
    }

    // Stage 1a: Pure-Omni verifier (`pure/verifier_engine.omni` with API rules 1010..1013)
    if let Err(e) = pure_default_verify_u16(candidate_u16, true) {
        return Ok(make_reject(
            6,
            &format!("PURE_VERIFIER_REJECT: {}", e),
            None,
            None,
            None,
            Some(0),
            Some(0),
            vec!["preserve_cap_mask", "restore_linear_drop", "sanitize_taint_sink"],
        ));
    }

    // Stage 1b: Frozen Rust verifier cross-check
    let cand_mod = match decode_omni_token_stream(candidate_u16) {
        Ok(m) => m,
        Err(e) => {
            return Ok(make_reject(
                6,
                &format!("DECODE_REJECT: {}", e),
                None,
                None,
                None,
                None,
                Some(0),
                vec!["fix_token_framing"],
            ));
        }
    };
    for f in &cand_mod.functions {
        let rep = verify_binary_function(f);
        if !rep.passed {
            return Ok(make_reject(
                6,
                &format!("RUST_VERIFIER_REJECT: {:?}", rep.errors),
                None,
                None,
                None,
                Some(f.fn_idx),
                Some(0),
                vec!["preserve_cap_mask", "fix_ssa_contiguity", "restore_linear_drop"],
            ));
        }
    }

    // Stage 1c: M2-multi ELF64 lowering + semantic equivalence on VM (and native Linux x86-64)
    let cand_elf = match compile_pure_u16_to_x86_64_elf_multi(
        candidate_u16,
        inputs[0],
        inputs[1],
        inputs[2],
    ) {
        Ok(e) => e,
        Err(e) => {
            return Ok(make_reject(
                7,
                &format!("M2_EMIT_REJECT: {}", e),
                Some(inputs),
                Some(expected_ret),
                None,
                Some(0),
                None,
                vec!["restrict_to_m2_subset"],
            ));
        }
    };

    let cand_entry = cand_mod.functions.last().map(|f| f.fn_idx).unwrap_or(0);
    let n_params = cand_mod.functions.last().map(|f| f.params.len()).unwrap_or(0);
    let vm_args: Vec<OmniValue> = inputs.iter().take(n_params).map(|&v| OmniValue::Int(v)).collect();

    // Stage 3: 5-run sequential consensus measurement with 0-variance deterministic verification
    let mut step_runs = [0i64; 5];
    for run in 0..5 {
        let mut vm = OmniBinaryVm::new();
        let val = match vm.execute_function(&cand_mod, cand_entry, vm_args.clone()) {
            Ok(OmniValue::Int(v)) => v,
            Ok(OmniValue::Trit(t)) => t as i64,
            Ok(other) => {
                return Ok(make_reject(
                    8,
                    &format!("SEMANTIC_TYPE_MISMATCH: got {:?}", other),
                    Some(inputs),
                    Some(expected_ret),
                    None,
                    Some(cand_entry),
                    None,
                    vec!["preserve_return_type"],
                ));
            }
            Err(e) => {
                return Ok(make_reject(
                    8,
                    &format!("VM_TRAP: {}", e),
                    Some(inputs),
                    Some(expected_ret),
                    None,
                    Some(cand_entry),
                    None,
                    vec!["revert_invalid_peephole", "check_bounds"],
                ));
            }
        };
        if val != expected_ret {
            return Ok(make_reject(
                8,
                "SEMANTIC_MISMATCH: candidate output diverged from expected",
                Some(inputs),
                Some(expected_ret),
                Some(val),
                Some(cand_entry),
                None,
                vec!["revert_invalid_peephole", "preserve_live_operands"],
            ));
        }
        step_runs[run] = vm.steps_executed as i64;
    }
    if step_runs.iter().any(|&s| s != step_runs[0]) {
        return Ok(make_reject(
            9,
            "NON_DETERMINISTIC_STEPS: vm_steps varied across 5 runs",
            Some(inputs),
            Some(step_runs[0]),
            None,
            Some(cand_entry),
            None,
            vec!["eliminate_nondeterminism"],
        ));
    }
    let cand_steps = step_runs[0];

    // Native Linux x86-64 parity check on emitted ELF64
    #[cfg(all(unix, target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let elf_path = format!(
            "/tmp/omni_prop_cand_{}_{}.elf",
            std::process::id(),
            &cand_sha[..10]
        );
        std::fs::write(&elf_path, &cand_elf).map_err(|e| e.to_string())?;
        let mut perms = std::fs::metadata(&elf_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&elf_path, perms).map_err(|e| e.to_string())?;
        let native_out = std::process::Command::new(&elf_path).output();
        let _ = std::fs::remove_file(&elf_path);
        let out = native_out.map_err(|e| format!("native candidate ELF spawn: {}", e))?;
        let got_i64 = if out.stdout.len() == 8 {
            Some(i64::from_le_bytes([
                out.stdout[0],
                out.stdout[1],
                out.stdout[2],
                out.stdout[3],
                out.stdout[4],
                out.stdout[5],
                out.stdout[6],
                out.stdout[7],
            ]))
        } else {
            String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().ok()
        };
        if !out.status.success() || got_i64 != Some(expected_ret) {
            return Ok(make_reject(
                8,
                &format!(
                    "NATIVE_ELF_MISMATCH: expected {}, got {:?} (stdout_len={}, rc={:?})",
                    expected_ret,
                    got_i64,
                    out.stdout.len(),
                    out.status.code()
                ),
                Some(inputs),
                Some(expected_ret),
                got_i64,
                Some(cand_entry),
                None,
                vec!["revert_invalid_peephole"],
            ));
        }
    }

    // Stage 4: Judge via Pure-Omni `pure/m7/judge.omni::p7_judge_metrics4` + `p7_adopt_rule`
    // Deterministic metrics use noise band = 0 (bc_words, elf_bytes, vm_steps).
    let judge_src = std::fs::read_to_string("pure/m7/judge.omni")
        .map_err(|e| format!("read pure/m7/judge.omni: {}", e))?;
    let judge_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&judge_src))?;
    let judge_mod = decode_omni_token_stream(&judge_u16)?;
    let mut jvm = OmniBinaryVm::new();

    // Index functions in judge.omni by declaration order (p7_cmp_band = 3, p7_combine2 = 6, p7_adopt_rule = 8)
    let cmp_band = |vm: &mut OmniBinaryVm, base: i64, cand: i64, band: i64| -> Result<i8, String> {
        match vm.execute_function(
            &judge_mod,
            3,
            vec![OmniValue::Int(base), OmniValue::Int(cand), OmniValue::Int(band)],
        )? {
            OmniValue::Trit(t) => Ok(t),
            other => Err(format!("p7_cmp_band unexpected {:?}", other)),
        }
    };
    let combine2 = |vm: &mut OmniBinaryVm, a: i8, b: i8| -> Result<i8, String> {
        match vm.execute_function(
            &judge_mod,
            6,
            vec![OmniValue::Trit(a), OmniValue::Trit(b)],
        )? {
            OmniValue::Trit(t) => Ok(t),
            other => Err(format!("p7_combine2 unexpected {:?}", other)),
        }
    };
    let adopt_rule = |vm: &mut OmniBinaryVm, gates_ok: i64, suite_v: i8| -> Result<i8, String> {
        match vm.execute_function(
            &judge_mod,
            8,
            vec![OmniValue::Int(gates_ok), OmniValue::Trit(suite_v)],
        )? {
            OmniValue::Trit(t) => Ok(t),
            other => Err(format!("p7_adopt_rule unexpected {:?}", other)),
        }
    };

    let v_words = cmp_band(&mut jvm, baseline_u16.len() as i64, candidate_u16.len() as i64, 0)?;
    let v_elf = cmp_band(&mut jvm, base_elf.len() as i64, cand_elf.len() as i64, 0)?;
    let v_steps = cmp_band(&mut jvm, base_steps, cand_steps, 0)?;
    let v_12 = combine2(&mut jvm, v_words, v_elf)?;
    let v_suite = combine2(&mut jvm, v_12, v_steps)?;
    let v_final = adopt_rule(&mut jvm, 1, v_suite)?;

    let verdict = match v_final {
        1 => ProposerVerdict::Adopt,
        0 => ProposerVerdict::Hold,
        _ => ProposerVerdict::Reject,
    };

    // Always produce Step 4 Audit View on ADOPT (and on HOLD/REJECT when valid)
    let dec = decompile_u16_stream_to_m3(candidate_u16)?;
    let diff = summarize_proposer_diff(baseline_u16, candidate_u16, "baseline", "candidate")?;

    Ok(ProposerCandidateOutcome {
        attempt,
        candidate_sha256: cand_sha,
        target: ProposerTargetSubsystem::Tier1Tokens,
        verdict,
        baseline_words: baseline_u16.len(),
        candidate_words: candidate_u16.len(),
        baseline_elf_bytes: base_elf.len(),
        candidate_elf_bytes: cand_elf.len(),
        baseline_vm_steps: base_steps,
        candidate_vm_steps: cand_steps,
        gradient: if verdict == ProposerVerdict::Reject {
            Some(ProposerGradient {
                err_code: 9,
                assertion_violated: String::from("BENCH_REGRESSION: deterministic metric regressed (-1t)"),
                minimal_failing_input: Some(inputs),
                expected_output: Some(expected_ret),
                actual_output: Some(expected_ret),
                localized_fn_idx: Some(cand_entry),
                localized_word_offset: None,
                ranked_repair_templates: vec!["eliminate_bloat", "fold_dead_identity_op"],
            })
        } else {
            None
        },
        audit_decompile: Some(dec.m3_source),
        audit_diff_summary: Some(diff.render_plain_language()),
        git_commit_sha: None,
    })
}

/// Autonomous Tier-1 peephole proposer pass: eliminates provably redundant identity instructions
/// (`vK = IADD x 0`, `vK = IMUL x 1`) by remapping downstream reads of `vK` to `x` and
/// re-packing dense contiguous SSA registers (`%0..%N-1`).
/// Untrusted by the TCB: its output MUST still pass Stages 0–4 before adoption.
pub fn proposer_tier1_peephole_identity_elim(stream: &[u16]) -> Result<Vec<u16>, String> {
    audit_validate_framing(stream)?;
    let m = decode_omni_token_stream(stream)?;
    if m.functions.len() != 1 {
        return Ok(stream.to_vec());
    }
    let f = &m.functions[0];
    // Identify identity instructions `dst = IADD op 0` or `dst = IMUL op 1` where `dst` is never
    // written more than once (pure SSA temp) and has no labels/branches in the function.
    let has_cf = f
        .instructions
        .iter()
        .any(|ins| matches!(ins.op_tok, 0x7F40 | 0x7F41 | 0x7F42 | 0x7F43 | 0x7F44));
    if has_cf {
        return Ok(stream.to_vec());
    }
    let mut write_counts: BTreeMap<u16, usize> = BTreeMap::new();
    for ins in &f.instructions {
        if let Some(d) = ins.dest_reg {
            *write_counts.entry(d).or_insert(0) += 1;
        }
    }

    let mut alias_map: BTreeMap<u16, BinOperand> = BTreeMap::new();
    let mut kept_insns: Vec<BinInstruction> = Vec::with_capacity(f.instructions.len());
    for ins in &f.instructions {
        let mut rewritten_ops = Vec::with_capacity(ins.operands.len());
        for op in &ins.operands {
            if let BinOperand::Reg(r) = op {
                if let Some(repl) = alias_map.get(r) {
                    rewritten_ops.push(repl.clone());
                    continue;
                }
            }
            rewritten_ops.push(op.clone());
        }
        let is_single_write = ins
            .dest_reg
            .map(|d| write_counts.get(&d).copied().unwrap_or(0) == 1)
            .unwrap_or(false);
        if is_single_write && rewritten_ops.len() == 2 {
            let is_add_zero = ins.op_tok == 0x7F10
                && matches!(rewritten_ops[1], BinOperand::ImmInt(0));
            let is_mul_one = ins.op_tok == 0x7F12
                && matches!(rewritten_ops[1], BinOperand::ImmInt(1));
            if (is_add_zero || is_mul_one)
                && matches!(rewritten_ops[0], BinOperand::Reg(_) | BinOperand::Param(_))
            {
                alias_map.insert(ins.dest_reg.unwrap(), rewritten_ops[0].clone());
                continue;
            }
        }
        let mut new_ins = ins.clone();
        new_ins.operands = rewritten_ops;
        kept_insns.push(new_ins);
    }

    if alias_map.is_empty() {
        return Ok(stream.to_vec());
    }

    // Re-number remaining destination registers densely `%0..%K-1` for SSA contiguity.
    let mut reg_remap: BTreeMap<u16, u16> = BTreeMap::new();
    let mut next_reg = REG_BASE;
    for ins in &mut kept_insns {
        for op in &mut ins.operands {
            if let BinOperand::Reg(r) = op {
                if let Some(&mapped) = reg_remap.get(r) {
                    *r = mapped;
                }
            }
        }
        if let Some(d) = ins.dest_reg {
            let mapped = *reg_remap.entry(d).or_insert_with(|| {
                let r = next_reg;
                next_reg += 1;
                r
            });
            ins.dest_reg = Some(mapped);
        }
    }

    // Re-encode packets via `pack_fn_module`.
    let encode_op = |pkt: &mut Vec<u16>, op: &BinOperand| -> Result<(), String> {
        match op {
            BinOperand::Reg(r) => pkt.push(*r),
            BinOperand::Param(p) => pkt.push(*p),
            BinOperand::Label(l) => pkt.push(*l),
            BinOperand::ImmTrit(t) => pkt.push(match *t {
                -1 => TOK_TRIT_NEG,
                0 => TOK_TRIT_ZERO,
                _ => TOK_TRIT_POS,
            }),
            BinOperand::ImmInt(v) => {
                if *v >= i16::MIN as i64 && *v <= i16::MAX as i64 {
                    pkt.push(TOK_IMM_I16);
                    pkt.push((*v as i16) as u16);
                } else {
                    let u = *v as u32;
                    pkt.push(TOK_IMM_I32);
                    pkt.push(((u >> 16) & 0xFFFF) as u16);
                    pkt.push((u & 0xFFFF) as u16);
                }
            }
            BinOperand::FnRef(id) => {
                pkt.push(TOK_SYM_FN);
                pkt.push(*id);
            }
            _ => return Err(format!("peephole: unsupported operand {:?}", op)),
        }
        Ok(())
    };

    let mut pkts: Vec<Vec<u16>> = Vec::with_capacity(kept_insns.len());
    for ins in &kept_insns {
        let mut pkt = Vec::new();
        if let Some(d) = ins.dest_reg {
            pkt.push(d);
            pkt.push(ins.op_tok);
            pkt.push(ins.ty_tok);
            pkt.push(ins.tc_tok);
        } else {
            pkt.push(ins.op_tok);
        }
        for op in &ins.operands {
            encode_op(&mut pkt, op)?;
        }
        pkt.push(TOK_INS_END);
        pkts.push(pkt);
    }

    Ok(pack_fn_module(
        &[],
        f.fn_idx,
        &f.params,
        f.ret_ty_tok,
        f.cap_mask,
        &pkts,
    ))
}

/// Step 5 permanent selftest gate (`omnic proposer-loop`):
/// Proves the entire closed-loop proposer architecture (`spec/m7/PROPOSER_LOOP_DESIGN.md`):
///   1. Stage 0 scope guard: allowlist passes M2/M3/M4, rejects mixed-subsystem paths, and triggers
///      `FATAL_ABORT` (`err_code=10`, `VERIFIER_TAMPER`) on any immutable deny-list path (`verifier_engine.omni`, `judge.omni`).
///   2. Stage 1 & CEGIS gradient: rejects broken-semantics (`SEMANTIC_MISMATCH` with minimal failing input)
///      and unauthorized-capability (`PURE_VERIFIER_REJECT`) proposals without adopting.
///   3. Stage 2 canary proof (`selftest_p7_0a_canary`) holds `+1t`/`0t` + 5 frozen hashes intact.
///   4. Stage 4 balanced-ternary verdicts via Pure-Omni `pure/m7/judge.omni`:
///      - Neutral candidate -> `0t (HOLD)` (discarded, NOT adopted)
///      - Regressing/bloated candidate -> `-1t (REJECT)` + `heal.omni` rollback + `learn.omni` `-3`
///      - Real autonomous Tier-1 peephole improvement -> `+1t (ADOPT)` (strictly fewer words, smaller ELF64,
///        fewer VM steps, 0-variance across 5 runs, native Linux x86-64 parity)
///   5. Step 4 Audit View readability: adopted change decompiles (`is_exact == true`) and produces
///      a reproducible plain-language proposer diff summary (`delta_words < 0`, `cap_escalation=false`).
///   6. Git-backed SHA-256 content-addressed store: auto-commits adopted candidate + audit view to
///      `flywheel/auto` via isolated `agent/tier1-peephole` branch, and sleep-time GC prunes unadopted scratch.
pub fn selftest_proposer_loop() -> Result<usize, String> {
    let mut checks = 0usize;

    // 1. Stage 0 TCB Scope Guard & Immutable Deny-List
    let guard = ProposerScopeGuard::snapshot(16_384, 0)?;
    guard
        .verify_deny_digests_intact()
        .map_err(|(_, m)| m)?;
    // Allowlisted single-subsystem paths pass
    guard
        .check_paths(ProposerTargetSubsystem::M2Emitter, &["pure/m2/emit_multi.omni"])
        .map_err(|(_, m)| m)?;
    guard
        .check_paths(ProposerTargetSubsystem::M3Frontend, &["pure/m3/lex.omni"])
        .map_err(|(_, m)| m)?;
    // Mixed-subsystem path rejected with SCOPE_VIOLATION
    match guard.check_paths(ProposerTargetSubsystem::M2Emitter, &["pure/m3/lex.omni"]) {
        Err((2, _)) => checks += 1,
        other => return Err(format!("proposer-loop: expected SCOPE_VIOLATION (2), got {:?}", other)),
    }
    // Immutable deny-list paths trigger FATAL_ABORT (err_code = 10, VERIFIER_TAMPER)
    for &denied in &["pure/verifier_engine.omni", "pure/m7/judge.omni", "pure/m7/heal.omni", "omni_rust_bootstrap.rs"] {
        match guard.check_paths(ProposerTargetSubsystem::M3Frontend, &[denied]) {
            Err((10, msg)) if msg.contains("VERIFIER_TAMPER") => {}
            other => {
                return Err(format!(
                    "proposer-loop: deny-listed '{}' did not trigger VERIFIER_TAMPER (10): {:?}",
                    denied, other
                ));
            }
        }
    }
    checks += 1;

    // 2. Stage 2 Canary Proof (ruler must be unbroken before judging any proposal)
    //    selftest_p7_0a_canary returns (checks, saved_bytes, attr_id [2=M2_EMITTER], neutral_attr [0=NONE], native_ok)
    //    and internally fails closed if canary != +1t, neutral != 0t, or frozen hashes drift.
    let (_canary_checks, saved_bytes, attr_id, neutral_attr, native_ok) = selftest_p7_0a_canary()?;
    let canary_ok = saved_bytes > 0 && attr_id == 2 && neutral_attr == 0 && native_ok;
    if !canary_ok {
        return Err(format!(
            "proposer-loop: Stage 2 canary failed (saved={}, attr={}, neutral_attr={}, native={})",
            saved_bytes, attr_id, neutral_attr, native_ok
        ));
    }
    checks += 1;

    // Baseline program with redundant identity ops (`+ 0`, `* 1`) for autonomous Tier-1 peephole optimization
    let baseline_src = "#[strict]\nfn main(x: i64, y: i64) -> i64 {\n    let a: i64 = x * 3;\n    let a_id: i64 = a + 0;\n    let b: i64 = y + 7;\n    let b_id: i64 = b * 1;\n    let total: i64 = a_id + b_id;\n    return total;\n}\n";
    let baseline_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(baseline_src))?;
    let inputs = [10i64, 5, 0];
    let expected_ret = 42i64; // (10*3) + (5+7) = 42

    // Initialize isolated Git-backed SHA-256 store in /tmp (never touches main repo!)
    let store_dir = std::path::PathBuf::from(format!(
        "/tmp/omni_proposer_store_{}",
        std::process::id()
    ));
    let store = ProposerGitStore::init(&store_dir)?;

    // 3. Negative Gate A: Broken semantics (+1 bug) MUST REJECT with CEGIS gradient & minimal failing input
    let broken_src = "#[strict]\nfn main(x: i64, y: i64) -> i64 {\n    let a: i64 = x * 3;\n    let b: i64 = y + 8;\n    return a + b;\n}\n";
    let broken_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(broken_src))?;
    let broken_out = proposer_evaluate_tier1_candidate(
        &guard,
        &baseline_u16,
        &broken_u16,
        inputs,
        expected_ret,
        canary_ok,
        0,
    )?;
    if broken_out.verdict != ProposerVerdict::Reject {
        return Err(format!(
            "proposer-loop: broken semantics candidate was not rejected: {:?}",
            broken_out.verdict
        ));
    }
    let grad = broken_out
        .gradient
        .as_ref()
        .ok_or_else(|| String::from("proposer-loop: missing CEGIS gradient on semantic rejection"))?;
    if !grad.assertion_violated.contains("SEMANTIC_MISMATCH")
        || grad.minimal_failing_input != Some(inputs)
        || grad.expected_output != Some(42)
        || grad.actual_output != Some(43)
        || grad.ranked_repair_templates.is_empty()
    {
        return Err(format!("proposer-loop: malformed CEGIS gradient: {:?}", grad));
    }
    store.record_scratch(&broken_out.candidate_sha256, &broken_u16)?;
    checks += 1;

    // 4. Negative Gate B: Unauthorized capability escalation MUST REJECT at Stage 0/1
    let cap_esc_src = "#[strict]\n#[cap(AI)]\nfn main(x: i64, y: i64) -> i64 {\n    let g: trit = egate(x - y);\n    return x * 3 + y + 7 + g * 0;\n}\n";
    let cap_esc_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(cap_esc_src))?;
    let cap_out = proposer_evaluate_tier1_candidate(
        &guard,
        &baseline_u16,
        &cap_esc_u16,
        inputs,
        expected_ret,
        canary_ok,
        1,
    )?;
    if cap_out.verdict != ProposerVerdict::Reject
        || !cap_out
            .gradient
            .as_ref()
            .map(|g| g.assertion_violated.contains("CAP_BUDGET_EXCEEDED"))
            .unwrap_or(false)
    {
        return Err(format!(
            "proposer-loop: capability escalation candidate was not rejected at Stage 0: {:?}",
            cap_out
        ));
    }
    checks += 1;

    // 5. Negative Gate C: Neutral candidate MUST HOLD (0t, never adopted)
    let hold_out = proposer_evaluate_tier1_candidate(
        &guard,
        &baseline_u16,
        &baseline_u16,
        inputs,
        expected_ret,
        canary_ok,
        2,
    )?;
    if hold_out.verdict != ProposerVerdict::Hold {
        return Err(format!(
            "proposer-loop: neutral candidate expected HOLD (0t), got {:?}",
            hold_out.verdict
        ));
    }
    store.record_scratch(&hold_out.candidate_sha256, &baseline_u16)?;
    checks += 1;

    // 6. Negative Gate D: Bloated/regressing candidate MUST REJECT (-1t)
    let bloated_src = "#[strict]\nfn main(x: i64, y: i64) -> i64 {\n    let a: i64 = x * 3;\n    let a1: i64 = a + 0;\n    let a2: i64 = a1 + 0;\n    let a3: i64 = a2 + 0;\n    let b: i64 = y + 7;\n    let b1: i64 = b * 1;\n    let total: i64 = a3 + b1;\n    return total;\n}\n";
    let bloated_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(bloated_src))?;
    let bloat_out = proposer_evaluate_tier1_candidate(
        &guard,
        &baseline_u16,
        &bloated_u16,
        inputs,
        expected_ret,
        canary_ok,
        3,
    )?;
    if bloat_out.verdict != ProposerVerdict::Reject {
        return Err(format!(
            "proposer-loop: regressing candidate expected REJECT (-1t), got {:?}",
            bloat_out.verdict
        ));
    }
    checks += 1;

    // 7. Positive Closed-Loop Proof: Autonomous proposer emits real improvement -> Stage 0..4 -> +1t ADOPT
    //    -> readable via Step 4 Audit View -> auto-committed to `flywheel/auto` -> sleep-time GC.
    let cand_u16 = proposer_tier1_peephole_identity_elim(&baseline_u16)?;
    let mut adopt_out = proposer_evaluate_tier1_candidate(
        &guard,
        &baseline_u16,
        &cand_u16,
        inputs,
        expected_ret,
        canary_ok,
        4,
    )?;
    if adopt_out.verdict != ProposerVerdict::Adopt
        || adopt_out.candidate_words >= adopt_out.baseline_words
        || adopt_out.candidate_elf_bytes >= adopt_out.baseline_elf_bytes
        || adopt_out.candidate_vm_steps >= adopt_out.baseline_vm_steps
    {
        return Err(format!(
            "proposer-loop: autonomous improvement did not ADOPT with real gains: {:?}",
            adopt_out
        ));
    }
    let audit_dec = adopt_out
        .audit_decompile
        .as_ref()
        .ok_or_else(|| String::from("proposer-loop: adopted candidate missing Step 4 decompile"))?;
    let audit_diff = adopt_out
        .audit_diff_summary
        .as_ref()
        .ok_or_else(|| String::from("proposer-loop: adopted candidate missing Step 4 diff summary"))?;
    if !audit_dec.contains("RECONSTRUCTED (exact)")
        || !audit_diff.contains("escalation=false")
        || !audit_diff.contains("hash_changed=true")
    {
        return Err(format!(
            "proposer-loop: Step 4 audit view invalid on adopted candidate:\n{}\n{}",
            audit_dec, audit_diff
        ));
    }
    let commit_sha = store.adopt_commit(
        "tier1-peephole",
        &adopt_out.candidate_sha256,
        &cand_u16,
        audit_dec,
        audit_diff,
    )?;
    adopt_out.git_commit_sha = Some(commit_sha.clone());
    if commit_sha.is_empty() {
        return Err(String::from("proposer-loop: empty git commit sha on flywheel/auto"));
    }
    // Verify sleep-time GC prunes unadopted scratch while preserving committed candidate
    let pruned = store.sleep_time_gc()?;
    let committed_bc = store
        .root
        .join("candidates")
        .join(format!("{}.omni_bc", adopt_out.candidate_sha256));
    if pruned < 2 || !committed_bc.is_file() {
        return Err(format!(
            "proposer-loop: sleep-time GC failed (pruned={}, committed_exists={})",
            pruned,
            committed_bc.is_file()
        ));
    }
    // Clean up temp store after verification
    let _ = std::fs::remove_dir_all(&store_dir);
    checks += 1;

    // 8. Wire Pure-Omni `heal.omni::p7_heal_decide` (on reject) + `learn.omni::p7_learn_update`
    //    (-3 on reject, 0 on hold, +2 on adopt) and verify tamper-checked checksum intact.
    let heal_src = std::fs::read_to_string("pure/m7/heal.omni")
        .map_err(|e| format!("read pure/m7/heal.omni: {}", e))?;
    let heal_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&heal_src))?;
    let heal_mod = decode_omni_token_stream(&heal_u16)?;
    let mut hvm = OmniBinaryVm::new();
    // p7_heal_decide(err_code=10 VERIFIER_TAMPER, attempt=0, max=3, rollback_ok=1) -> 3 (FATAL_ABORT)
    let act_tamper = match hvm.execute_function(
        &heal_mod,
        3,
        vec![OmniValue::Int(10), OmniValue::Int(0), OmniValue::Int(3), OmniValue::Int(1)],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("p7_heal_decide unexpected {:?}", other)),
    };
    // p7_heal_decide(err_code=3 healable SEMANTIC_MISMATCH, attempt=0, max=3, rollback_ok=1) -> 1 (ROLLBACK_AND_RETRY)
    let act_retry = match hvm.execute_function(
        &heal_mod,
        3,
        vec![OmniValue::Int(3), OmniValue::Int(0), OmniValue::Int(3), OmniValue::Int(1)],
    )? {
        OmniValue::Int(v) => v,
        other => return Err(format!("p7_heal_decide unexpected {:?}", other)),
    };
    if act_tamper != 3 || act_retry != 1 {
        return Err(format!(
            "proposer-loop: heal.omni decision mismatch (tamper={}, retry={})",
            act_tamper, act_retry
        ));
    }

    let judge_src = std::fs::read_to_string("pure/m7/judge.omni")
        .map_err(|e| format!("read pure/m7/judge.omni: {}", e))?;
    let judge_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&judge_src))?;
    let n_judge = decode_omni_token_stream(&judge_u16)?.functions.len() as u16;
    let learn_src = std::fs::read_to_string("pure/m7/learn.omni")
        .map_err(|e| format!("read pure/m7/learn.omni: {}", e))?;
    let learn_combined = format!("{}\n{}", judge_src, learn_src);
    let learn_u16 = with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(&learn_combined))?;
    let learn_mod = decode_omni_token_stream(&learn_u16)?;
    let mut lvm = OmniBinaryVm::new();
    // learn.omni functions follow judge.omni:
    // +1=p7_learn_stamp_checksum, +2=p7_learn_init, +3=p7_learn_verify, +4=p7_learn_update
    let fn_learn_init = n_judge + 2;
    let fn_learn_verify = n_judge + 3;
    let fn_learn_update = n_judge + 4;
    let mut ltbl = match lvm.execute_function(&learn_mod, fn_learn_init, vec![OmniValue::Bytes(vec![0u8; 512])])? {
        OmniValue::Bytes(b) => b,
        other => return Err(format!("p7_learn_init unexpected {:?}", other)),
    };
    for &verdict_trit in &[-1i8, 0i8, 1i8] {
        ltbl = match lvm.execute_function(
            &learn_mod,
            fn_learn_update,
            vec![
                OmniValue::Bytes(ltbl),
                OmniValue::Int(1),
                OmniValue::Int(0),
                OmniValue::Trit(verdict_trit),
            ],
        )? {
            OmniValue::Bytes(b) => b,
            other => return Err(format!("p7_learn_update unexpected {:?}", other)),
        };
    }
    let learn_ok = match lvm.execute_function(&learn_mod, fn_learn_verify, vec![OmniValue::Bytes(ltbl.clone())])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("p7_learn_verify unexpected {:?}", other)),
    };
    if learn_ok != 3 {
        return Err(format!("proposer-loop: learn.omni checksum verify expected 3 updates, got {}", learn_ok));
    }
    // Negative control: 1-byte tamper in experience table must fail closed (-1).
    let mut bad_ltbl = ltbl;
    bad_ltbl[64] ^= 0xFF;
    let learn_tamper = match lvm.execute_function(&learn_mod, fn_learn_verify, vec![OmniValue::Bytes(bad_ltbl)])? {
        OmniValue::Int(v) => v,
        other => return Err(format!("p7_learn_verify tamper unexpected {:?}", other)),
    };
    if learn_tamper != -1 {
        return Err(format!("proposer-loop: tampered learn.omni table did not fail closed ({})", learn_tamper));
    }
    checks += 1;

    // 9. Final post-episode deny-list digest verification (0 tampering across entire run)
    guard
        .verify_deny_digests_intact()
        .map_err(|(_, m)| m)?;
    checks += 1;

    Ok(checks)
}

/// CLI handler for `omnic propose-run [--store <dir>]`: runs a live autonomous Tier-1 proposer
/// episode, auto-commits the `+1t` adopted improvement to `flywheel/auto`, and prints the Step 4
/// audit view (decompiled M3 + plain-language diff summary).
pub fn run_omni_propose_run_cli(argv: &[String]) -> i32 {
    let mut store_dir = std::path::PathBuf::from(format!(
        "/tmp/omni_proposer_live_{}",
        std::process::id()
    ));
    let mut i = 0;
    while i < argv.len() {
        if argv[i] == "--store" && i + 1 < argv.len() {
            store_dir = std::path::PathBuf::from(&argv[i + 1]);
            i += 2;
        } else {
            i += 1;
        }
    }
    let guard = match ProposerScopeGuard::snapshot(16_384, 0) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] {}", e);
            return 1;
        }
    };
    let canary_ok = match selftest_p7_0a_canary() {
        Ok((_, saved, attr, neut_attr, nat)) => saved > 0 && attr == 2 && neut_attr == 0 && nat,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] canary: {}", e);
            return 1;
        }
    };
    let baseline_src = "#[strict]\nfn main(x: i64, y: i64) -> i64 {\n    let a: i64 = x * 3;\n    let a_id: i64 = a + 0;\n    let b: i64 = y + 7;\n    let b_id: i64 = b * 1;\n    let total: i64 = a_id + b_id;\n    return total;\n}\n";
    let baseline_u16 = match with_lockstep_suspended(|| compile_omni_surface_strict_to_u16(baseline_src)) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] {}", e);
            return 1;
        }
    };
    let cand_u16 = match proposer_tier1_peephole_identity_elim(&baseline_u16) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] {}", e);
            return 1;
        }
    };
    let out = match proposer_evaluate_tier1_candidate(
        &guard,
        &baseline_u16,
        &cand_u16,
        [10, 5, 0],
        42,
        canary_ok,
        0,
    ) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] {}", e);
            return 1;
        }
    };
    if out.verdict != ProposerVerdict::Adopt {
        eprintln!("[PROPOSE-RUN] verdict={:?} (not adopted)", out.verdict);
        return 1;
    }
    let store = match ProposerGitStore::init(&store_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] store: {}", e);
            return 1;
        }
    };
    let dec = out.audit_decompile.as_deref().unwrap_or("");
    let diff = out.audit_diff_summary.as_deref().unwrap_or("");
    let sha = match store.adopt_commit("tier1-peephole", &out.candidate_sha256, &cand_u16, dec, diff) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[PROPOSE-RUN FAIL] commit: {}", e);
            return 1;
        }
    };
    println!(
        "[PROPOSE-RUN ADOPT +1t] commit={} branch=flywheel/auto store={} | words {}->{} | elf {}B->{}B | steps {}->{}",
        sha,
        store_dir.display(),
        out.baseline_words,
        out.candidate_words,
        out.baseline_elf_bytes,
        out.candidate_elf_bytes,
        out.baseline_vm_steps,
        out.candidate_vm_steps
    );
    print!("{}", diff);
    print!("{}", dec);
    0
}
