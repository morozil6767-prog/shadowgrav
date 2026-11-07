//! Neumaier compensated summation (Contract: all force/moment accumulations).
#[derive(Clone, Copy, Default, Debug)]
pub struct Neum { s: f64, c: f64 }
impl Neum {
    #[inline]
    pub fn add(&mut self, x: f64) {
        let t = self.s + x;
        if self.s.abs() >= x.abs() { self.c += (self.s - t) + x; }
        else { self.c += (x - t) + self.s; }
        self.s = t;
    }
    #[inline]
    pub fn value(&self) -> f64 { self.s + self.c }
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Neum3 { a: [Neum; 3] }
impl Neum3 {
    pub fn add(&mut self, v: &[f64; 3]) { for i in 0..3 { self.a[i].add(v[i]); } }
    pub fn add_scaled(&mut self, s: f64, v: &[f64; 3]) { for i in 0..3 { self.a[i].add(s * v[i]); } }
    pub fn value(&self) -> [f64; 3] { [self.a[0].value(), self.a[1].value(), self.a[2].value()] }
}
