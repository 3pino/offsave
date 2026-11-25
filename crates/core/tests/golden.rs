//! `tests/vectors/engine.json` の試験データ。Android の Kotlin 版（EngineTest）も同じファイルで試験する。

use offsave_core::Ms;
use offsave_core::engine::{Decision, Input, Verdict, decide};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    name: String,
    input: Input,
    expect: Expect,
}

#[derive(Deserialize)]
struct Expect {
    verdict: Verdict,
    new_graces: Vec<(String, Ms)>,
    next_wakeup: Option<Ms>,
}

#[test]
fn engine_vectors() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("vectors/engine.json")).expect("engine.json を読めません");
    assert!(cases.len() >= 10);
    for case in cases {
        let got = decide(&case.input);
        let want = Decision {
            verdict: case.expect.verdict,
            new_graces: case.expect.new_graces,
            next_wakeup: case.expect.next_wakeup,
        };
        assert_eq!(got, want, "ケース: {}", case.name);
    }
}
