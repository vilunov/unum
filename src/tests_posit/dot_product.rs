use crate::Posit;
use lazy_static::lazy_static;

lazy_static! {
    static ref BS: Vec<f64> = vec![5.0, 8.0, 12.0, 15.0, 20.0];
    static ref EPS: Posit = 10e-8.into();
    static ref RES: Posit = 8779.0.into();
}

fn init_v1(a: f64) -> Vec<Posit> {
    vec![
        10f64.powf(a),
        1223f64,
        10f64.powf(a - 1.0),
        10f64.powf(a - 2.0),
        3f64,
        -(10f64.powf(a - 5.0)),
    ]
    .into_iter()
    .map(<_>::into)
    .collect()
}

fn init_v2(a: f64) -> Vec<Posit> {
    vec![
        10f64.powf(a),
        2f64,
        -(10f64.powf(a + 1.0)),
        10f64.powf(a),
        2111f64,
        10f64.powf(a + 3.0),
    ]
    .into_iter()
    .map(<_>::into)
    .collect()
}

fn dot(left: Vec<Posit>, right: Vec<Posit>) -> Posit {
    let mut res = Posit::zero();
    for (a, b) in left.into_iter().zip(right.into_iter()) {
        dbg!(&a, &b);
        let prod = a * b;
        dbg!(&prod);
        res = res + prod;
        dbg!(&res);
    }
    res
}

#[test]
fn test_a5() {
    let x = init_v1(5.0);
    for &i in BS.iter() {
        dbg!(i);
        let y = init_v2(i);
        let dot = dot(x.clone(), y);
        println!("{:?}", dot);
        dbg!(&dot, RES.clone());
        assert!((dot - RES.clone()).abs() < EPS.clone());
    }
}

#[test]
fn test_a10() {
    let x = init_v1(10.0);
    for &i in BS.iter() {
        let y = init_v2(i);
        let dot = dot(x.clone(), y);
        println!("{:?}", dot);
        assert!((dot - RES.clone()).abs() < EPS.clone());
    }
}
