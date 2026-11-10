//! Gauss-Legendre nodes/weights on [-1,1] by Newton iteration.
use std::f64::consts::PI;
#[derive(Clone, Debug)]
pub struct Gl {
    pub x: Vec<f64>,
    pub w: Vec<f64>,
}

fn leg(n: usize, z: f64) -> (f64, f64) {
    let (mut p1, mut p2) = (1.0, 0.0);
    for j in 0..n {
        let p3 = p2;
        p2 = p1;
        p1 = ((2 * j + 1) as f64 * z * p2 - j as f64 * p3) / (j + 1) as f64;
    }
    let dp = n as f64 * (z * p1 - p2) / (z * z - 1.0);
    (p1, dp)
}
impl Gl {
    pub fn new(n: usize) -> Gl {
        let mut x = vec![0.0; n];
        let mut w = vec![0.0; n];
        for i in 0..n {
            let mut z = crate::mathx::cos(PI * (i as f64 + 0.75) / (n as f64 + 0.5));
            for _ in 0..100 {
                let (p, dp) = leg(n, z);
                let dz = p / dp;
                z -= dz;
                if dz.abs() < 1e-16 {
                    break;
                }
            }
            let (_, dp) = leg(n, z);
            x[i] = z;
            w[i] = 2.0 / ((1.0 - z * z) * dp * dp);
        }
        Gl { x, w }
    }
}
