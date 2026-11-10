//! Scenario files: a plain-JSON description of a run (bodies, globals, mode, quadrature, steps) that anyone can write by hand or
//! generate from Python, and that `sgrun run` turns into a `World`. Std-only (a small JSON reader lives here): no new dependency.
//!
//! Format `shadowgrav-scenario/1` (unknown keys are ERRORS, so a typo cannot silently fall back to a default):
//! ```json
//! { "format": "shadowgrav-scenario/1", "title": "test (c)",
//!   "mode": "v0",                       // v0 | v1 | v2            (default v0)
//!   "quadrature": "standard",           // standard | validation | xhi | certified   (default standard)
//!   "length_unit": "a0",                // a0 = radius of the m=1, rho=RHO0 sphere (0.01 in code units) | code   (default a0)
//!   "globals": { "tau_c": 20 },         // OR "kappa0"; optional g_n, xi, v_g, alpha_bg, w:[x,y,z], pi (uncalibrated mode)
//!   "bodies": [ {"x":[0,0,0]}, {"x":[10,0,0], "v":[0,0,0], "m":1, "rho":2.39e5} ],
//!   "step": { "h": 1e-4 },              // OR { "adaptive": { "h_max": 1e-3, "eta_t": 0.005 } }
//!   "run": { "steps": 0, "stride": 1 }, // steps = 0: evaluate forces at t = 0 only
//!   "leaf_budget": 0,                   // 0 = unlimited; exceeding aborts (never silent coarsening)
//!   "diagnostics": { "reference": false, "tau_max": false } }
//! ```
//! `tau_c` means the optical depth tau_c = 3 kappa m / (2 pi a^2) of the m = 1, rho = RHO0 reference sphere (as in the Spec test
//! problems); bodies with other m or rho get their own tau_c, reported in the output as `bodies[i].tau_c`.
use crate::derive::derive;
use crate::driver::{default_options, kappa0_for_tau, sphere_radius, RHO0};
use crate::geom::V3;
use crate::state::{SimConfig, StepControl, World};
use crate::types::*;
use std::collections::BTreeMap;

// ------------------------------------------------------------------------------------------------ minimal JSON
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}
impl<'a> P<'a> {
    fn err<T>(&self, m: &str) -> Result<T, String> {
        let line = self.s[..self.i.min(self.s.len())]
            .iter()
            .filter(|&&c| c == b'\n')
            .count()
            + 1;
        Err(format!(
            "JSON error at byte {} (line {}): {}",
            self.i, line, m
        ))
    }
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn lit(&mut self, w: &str, v: Json) -> Result<Json, String> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            self.err("invalid literal")
        }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > 64 {
            return self.err("nesting too deep");
        }
        self.ws();
        match self.s.get(self.i) {
            None => self.err("unexpected end of input"),
            Some(b'{') => {
                self.i += 1;
                let mut m = BTreeMap::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    self.ws();
                    let k = match self.value(depth + 1)? {
                        Json::Str(k) => k,
                        _ => return self.err("object key must be a string"),
                    };
                    self.ws();
                    if self.s.get(self.i) != Some(&b':') {
                        return self.err("expected ':'");
                    }
                    self.i += 1;
                    let v = self.value(depth + 1)?;
                    if m.insert(k.clone(), v).is_some() {
                        return self.err(&format!("duplicate key \"{k}\""));
                    }
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        _ => return self.err("expected ',' or '}'"),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(a));
                }
                loop {
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(a));
                        }
                        _ => return self.err("expected ',' or ']'"),
                    }
                }
            }
            Some(b'"') => {
                self.i += 1;
                let mut out: Vec<u8> = Vec::new();
                loop {
                    match self.s.get(self.i) {
                        None => return self.err("unterminated string"),
                        Some(b'"') => {
                            self.i += 1;
                            break;
                        }
                        Some(b'\\') => {
                            self.i += 1;
                            match self.s.get(self.i) {
                                Some(b'n') => out.push(b'\n'),
                                Some(b't') => out.push(b'\t'),
                                Some(b'r') => out.push(b'\r'),
                                Some(b'"') => out.push(b'"'),
                                Some(b'\\') => out.push(b'\\'),
                                Some(b'/') => out.push(b'/'),
                                Some(b'u') => {
                                    let h = std::str::from_utf8(
                                        self.s.get(self.i + 1..self.i + 5).unwrap_or(&[]),
                                    )
                                    .unwrap_or("");
                                    let cp = u32::from_str_radix(h, 16).map_err(|_| {
                                        format!("bad \\u escape at byte {}", self.i)
                                    })?;
                                    let ch = char::from_u32(cp).unwrap_or('\u{fffd}');
                                    let mut b = [0u8; 4];
                                    out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                                    self.i += 4;
                                }
                                _ => return self.err("bad escape"),
                            }
                            self.i += 1;
                        }
                        Some(&c) => {
                            out.push(c);
                            self.i += 1;
                        }
                    }
                }
                String::from_utf8(out)
                    .map(Json::Str)
                    .or_else(|_| self.err("invalid UTF-8 in string"))
            }
            Some(b't') => self.lit("true", Json::Bool(true)),
            Some(b'f') => self.lit("false", Json::Bool(false)),
            Some(b'n') => self.lit("null", Json::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let st = self.i;
                while self.i < self.s.len()
                    && matches!(
                        self.s[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap_or("");
                match t.parse::<f64>() {
                    Ok(v) if v.is_finite() => Ok(Json::Num(v)),
                    _ => {
                        self.i = st;
                        self.err("invalid or non-finite number")
                    }
                }
            }
            Some(_) => self.err("unexpected character"),
        }
    }
}
pub fn parse_json(text: &str) -> Result<Json, String> {
    let mut p = P {
        s: text.as_bytes(),
        i: 0,
    };
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return p.err("trailing characters after the JSON value");
    }
    Ok(v)
}

// ------------------------------------------------------------------------------------------------ typed access with strict keys
struct Obj<'a> {
    m: &'a BTreeMap<String, Json>,
    path: String,
    used: std::cell::RefCell<Vec<String>>,
}
impl<'a> Obj<'a> {
    fn of(j: &'a Json, path: &str) -> Result<Obj<'a>, String> {
        match j {
            Json::Obj(m) => Ok(Obj {
                m,
                path: path.to_string(),
                used: Default::default(),
            }),
            _ => Err(format!("{path}: expected an object")),
        }
    }
    fn get(&self, k: &str) -> Option<&'a Json> {
        self.used.borrow_mut().push(k.to_string());
        self.m.get(k)
    }
    fn num(&self, k: &str) -> Result<Option<f64>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(Json::Num(v)) => Ok(Some(*v)),
            Some(_) => Err(format!("{}.{k}: expected a number", self.path)),
        }
    }
    fn string(&self, k: &str) -> Result<Option<&'a str>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(Json::Str(s)) => Ok(Some(s)),
            Some(_) => Err(format!("{}.{k}: expected a string", self.path)),
        }
    }
    fn boolean(&self, k: &str) -> Result<Option<bool>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(Json::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(format!("{}.{k}: expected true or false", self.path)),
        }
    }
    fn v3(&self, k: &str) -> Result<Option<V3>, String> {
        match self.get(k) {
            None => Ok(None),
            Some(Json::Arr(a)) if a.len() == 3 => {
                let mut o = [0.0; 3];
                for (i, e) in a.iter().enumerate() {
                    match e {
                        Json::Num(v) => o[i] = *v,
                        _ => return Err(format!("{}.{k}[{i}]: expected a number", self.path)),
                    }
                }
                Ok(Some(o))
            }
            Some(_) => Err(format!("{}.{k}: expected [x, y, z]", self.path)),
        }
    }
    fn finish(&self) -> Result<(), String> {
        let used = self.used.borrow();
        for k in self.m.keys() {
            if !used.iter().any(|u| u == k) {
                return Err(format!("{}: unknown key \"{k}\" (known keys are listed in the scenario format documentation; a typo would otherwise be ignored silently)", self.path));
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------ scenario
#[derive(Clone, Debug)]
pub struct Scenario {
    pub title: String,
    pub mode: Mode,
    pub quadrature: String,
    pub validation: Option<Validation>,
    pub globals: Globals,
    pub bodies: Vec<(V3, V3, f64, f64)>, // x (code units), v, m, rho
    pub h: StepControl,
    pub steps: u64,
    pub stride: u64,
    pub leaf_budget: u64,
    pub want_reference: bool,
    pub want_tau_max: bool,
}

impl Scenario {
    pub fn from_json_text(text: &str) -> Result<Scenario, String> {
        let j = parse_json(text)?;
        let top = Obj::of(&j, "scenario")?;
        match top.string("format")? {
            Some("shadowgrav-scenario/1") => {}
            Some(o) => {
                return Err(format!(
                    "scenario.format: unsupported \"{o}\" (this build reads shadowgrav-scenario/1)"
                ))
            }
            None => {
                return Err("scenario.format: required, must be \"shadowgrav-scenario/1\"".into())
            }
        }
        let title = top.string("title")?.unwrap_or("").to_string();
        let mode = match top.string("mode")?.unwrap_or("v0") {
            "v0" => Mode::V0,
            "v1" => Mode::V1,
            "v2" => Mode::V2,
            o => return Err(format!("scenario.mode: \"{o}\" is not one of v0, v1, v2")),
        };
        let quadrature = top.string("quadrature")?.unwrap_or("standard").to_string();
        let validation = match quadrature.as_str() {
            "standard" => None,
            "validation" => Some(Validation::default_validation()),
            "xhi" => Some(Validation::xhi()),
            "certified" => Some(Validation::certified()),
            o => {
                return Err(format!(
                "scenario.quadrature: \"{o}\" is not one of standard, validation, xhi, certified"
            ))
            }
        };
        let a0 = sphere_radius(1.0, RHO0);
        let unit = match top.string("length_unit")?.unwrap_or("a0") {
            "a0" => a0,
            "code" => 1.0,
            o => {
                return Err(format!(
                    "scenario.length_unit: \"{o}\" is not one of a0, code"
                ))
            }
        };

        let mut g = Globals::defaults();
        let gj = top
            .get("globals")
            .ok_or("scenario.globals: required (give tau_c or kappa0)")?;
        let go = Obj::of(gj, "scenario.globals")?;
        let (tau, kap) = (go.num("tau_c")?, go.num("kappa0")?);
        g.kappa0 = match (tau, kap) {
            (Some(t), None) => {
                if crate::mathx::not_gt(t, 0.0) {
                    return Err("scenario.globals.tau_c must be > 0".into());
                }
                kappa0_for_tau(t, 1.0, RHO0)
            }
            (None, Some(k)) => k,
            _ => return Err("scenario.globals: give exactly one of tau_c or kappa0".into()),
        };
        if let Some(v) = go.num("g_n")? {
            g.g_n = v;
        }
        if let Some(v) = go.num("xi")? {
            g.xi = v;
        }
        if let Some(v) = go.num("v_g")? {
            g.v_g = v;
        }
        if let Some(v) = go.num("alpha_bg")? {
            g.alpha_bg = v;
        }
        if let Some(v) = go.v3("w")? {
            g.w = v;
        }
        if let Some(v) = go.num("pi")? {
            g.pi_override = Some(v);
        }
        go.finish()?;

        let bj = match top.get("bodies") {
            Some(Json::Arr(a)) if !a.is_empty() => a,
            _ => return Err("scenario.bodies: required, a non-empty array".into()),
        };
        let mut bodies = Vec::new();
        for (i, b) in bj.iter().enumerate() {
            let o = Obj::of(b, &format!("scenario.bodies[{i}]"))?;
            let x = o
                .v3("x")?
                .ok_or_else(|| format!("scenario.bodies[{i}].x: required"))?;
            let v = o.v3("v")?.unwrap_or([0.0; 3]);
            let m = o.num("m")?.unwrap_or(1.0);
            let rho = o.num("rho")?.unwrap_or(RHO0);
            o.finish()?;
            bodies.push(([x[0] * unit, x[1] * unit, x[2] * unit], v, m, rho));
        }

        let h = match top.get("step") {
            None => StepControl::Fixed { h: 1e-4 },
            Some(sj) => {
                let so = Obj::of(sj, "scenario.step")?;
                let h = so.num("h")?;
                let ad = so.get("adaptive");
                let r = match (h, ad) {
                    (Some(h), None) => StepControl::Fixed { h },
                    (None, Some(aj)) => {
                        let ao = Obj::of(aj, "scenario.step.adaptive")?;
                        let h_max = ao
                            .num("h_max")?
                            .ok_or("scenario.step.adaptive.h_max: required")?;
                        let eta_t = ao.num("eta_t")?.unwrap_or(0.005);
                        ao.finish()?;
                        StepControl::Adaptive { h_max, eta_t }
                    }
                    _ => return Err("scenario.step: give exactly one of h or adaptive".into()),
                };
                so.finish()?;
                r
            }
        };
        let (mut steps, mut stride) = (0u64, 1u64);
        if let Some(rj) = top.get("run") {
            let ro = Obj::of(rj, "scenario.run")?;
            let nn = |v: f64, what: &str| -> Result<u64, String> {
                if v >= 0.0 && v.fract() == 0.0 && v <= 1e12 {
                    Ok(v as u64)
                } else {
                    Err(format!(
                        "scenario.run.{what}: expected a non-negative integer"
                    ))
                }
            };
            if let Some(v) = ro.num("steps")? {
                steps = nn(v, "steps")?;
            }
            if let Some(v) = ro.num("stride")? {
                stride = nn(v, "stride")?;
                if stride == 0 {
                    return Err("scenario.run.stride must be >= 1".into());
                }
            }
            ro.finish()?;
        }
        let leaf_budget = match top.num("leaf_budget")? {
            None => 0,
            Some(v) if v >= 0.0 && v.fract() == 0.0 => v as u64,
            Some(_) => return Err("scenario.leaf_budget: expected a non-negative integer".into()),
        };
        let (mut want_reference, mut want_tau_max) = (false, false);
        if let Some(dj) = top.get("diagnostics") {
            let d = Obj::of(dj, "scenario.diagnostics")?;
            want_reference = d.boolean("reference")?.unwrap_or(false);
            want_tau_max = d.boolean("tau_max")?.unwrap_or(false);
            d.finish()?;
        }
        top.finish()?;
        Ok(Scenario {
            title,
            mode,
            quadrature,
            validation,
            globals: g,
            bodies,
            h,
            steps,
            stride,
            leaf_budget,
            want_reference,
            want_tau_max,
        })
    }

    /// Build the world. Every validation error of the engine (non-finite input, bad config) is returned as text.
    pub fn build(&self) -> Result<World, String> {
        let n = self.bodies.len();
        let p = Particles {
            x: self.bodies.iter().map(|b| b.0).collect(),
            v: self.bodies.iter().map(|b| b.1).collect(),
            m: self.bodies.iter().map(|b| b.2).collect(),
            rho: self.bodies.iter().map(|b| b.3).collect(),
            delta: vec![0.0; n],
            kind: vec![Kind::Body; n],
        };
        let mut opt = default_options(self.mode);
        opt.leaf_budget = self.leaf_budget;
        opt.validation = self.validation;
        let cfg = SimConfig {
            step: self.h,
            ..SimConfig::fixed(1e-4)
        };
        // surface derive errors early with the body index
        derive(&p, &self.globals).map_err(|e| format!("derive failed: {e:?}"))?;
        World::new(p, self.globals.clone(), opt, cfg).map_err(|e| format!("{e:?}"))
    }
}
