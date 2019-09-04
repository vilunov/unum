use lazy_static::lazy_static;

use crate::Posit;

const LIMIT: u32 = 10000000;
lazy_static! {
    static ref RES: Posit = Posit::from(4.0 / 3.0);
    static ref EPS: Posit = Posit::from(10e-8);
}

fn half_divide_method(
    mut a: Posit,
    mut b: Posit,
    stop: Posit,
    f: impl Fn(Posit) -> Posit,
    iterations: u32,
) -> Posit {
    let e: Posit = 0.5.into();
    let mut x: Posit = (a.clone() + b.clone()) / 2.0.into();
    let mut i = 0;
    while f(x.clone()) >= stop && i < iterations {
        x = (a.clone() + b.clone()) / 2.0.into();
        let f1 = f(x.clone() - e.clone());
        let f2 = f(x.clone() + e.clone());
        if f1 < f2 {
            b = x.clone();
        } else {
            a = x.clone();
        }
        i += 1;
    }
    (a + b) / 2.0.into()
}

fn parabola(x: Posit) -> Posit {
    (x - RES.clone()).pow(2)
}

#[test]
fn test_minima() {
    let minima = half_divide_method(Posit::from(-2.0), Posit::from(4.65), Posit::from(0.000001), parabola, LIMIT);
    let err = (RES.clone() - minima.clone()).abs();
    println!(
        "Minima: {:?}\nRes: {:?}\nError: {:?}",
        minima.clone(),
        &*RES,
        &err,
    );
    assert!(err < EPS.clone());
}
