//! Vector helpers, angles, ray-sphere chord (eq. 16).
pub type V3 = [f64; 3];
#[inline]
pub fn dot(a: &V3, b: &V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
pub fn sub(a: &V3, b: &V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
pub fn add(a: &V3, b: &V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
pub fn scale(a: &V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
pub fn cross(a: &V3, b: &V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[inline]
pub fn norm(a: &V3) -> f64 {
    dot(a, a).sqrt()
}
pub fn normalize(a: &V3) -> V3 {
    let n = norm(a);
    scale(a, 1.0 / n)
}

/// Angle between unit vectors, atan2 form (Architecture R6; never arccos).
#[inline]
pub fn psi(a: &V3, b: &V3) -> f64 {
    crate::mathx::atan2(norm(&cross(a, b)), dot(a, b))
}

/// Eq. 16. Ray x_k + s q (q unit), sphere centre offset c, radius a.
/// Returns (s1, s2, L) or None if b >= a or s2 <= 0. Target inside => s1<0<s2, L=s2.
#[inline]
pub fn chord(c: &V3, a: f64, q: &V3) -> Option<(f64, f64, f64)> {
    let t = dot(c, q);
    let p = [c[0] - t * q[0], c[1] - t * q[1], c[2] - t * q[2]]; // perpendicular part: avoids |c|^2-t^2 cancellation
    let b = norm(&p);
    if b >= a {
        return None;
    }
    let h = ((a - b) * (a + b)).sqrt();
    let (s1, s2) = (t - h, t + h);
    if s2 <= 0.0 {
        return None;
    }
    let l = if s1 > 0.0 { 2.0 * h } else { s2 };
    Some((s1, s2, l))
}

/// g(n; eps) = |n+eps| (n+eps)   (Spec 2.5)
#[inline]
pub fn gvec(n: &V3, eps: &V3) -> V3 {
    let v = add(n, eps);
    scale(&v, norm(&v))
}

/// Orthonormal pair (e1,e2) perpendicular to unit c.
pub fn orthobasis(c: &V3) -> (V3, V3) {
    let ax = if c[0].abs() <= c[1].abs() && c[0].abs() <= c[2].abs() {
        [1.0, 0.0, 0.0]
    } else if c[1].abs() <= c[2].abs() {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let e1 = normalize(&cross(c, &ax));
    let e2 = cross(c, &e1);
    (e1, e2)
}
