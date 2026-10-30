//! 用小码表检查搜索边界与候选约束，真实码表另做离线评测。
use super::{Decoder, Request, Segment};
use qingjian_lm::BigramModel;

fn decoder() -> Decoder {
    Decoder::from_parts("你\twq\t100\n是\tj\t100\n谁\tywyg\t100\n人\tw\t80\n今天\twygd\t100\n下午\tghtf\t100\n三点\tdghk\t100\n开会\tgawf\t100\n", BigramModel::default()).unwrap()
}

#[test]
fn mixed_short_and_full_codes_have_no_spurious_letters() {
    let result = decoder()
        .decode(Request {
            codes: "wqjywyg".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(result.candidates[0].text, "你是谁");
    assert_eq!(
        result.candidates[0]
            .segments
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>(),
        vec!["wq", "j", "ywyg"]
    );
}
#[test]
fn full_phrase_codes_form_one_sentence() {
    let result = decoder()
        .decode(Request {
            codes: "wygdghtfdghkgawf".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(result.candidates[0].text, "今天下午三点开会");
}
#[test]
fn unfinished_tail_is_not_invented_as_a_word() {
    let result = decoder()
        .decode(Request {
            codes: "wqjywy".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(!result.complete);
    assert_eq!(result.candidates[0].text, "你是");
    assert_eq!(result.candidates[0].pending, "ywy");
}
#[test]
fn locked_prefix_is_validated() {
    let r = Request {
        codes: "wqj".into(),
        locked: vec![Segment {
            code: "wq".into(),
            text: "错字".into(),
        }],
    };
    assert!(decoder().decode(r).is_err());
    let r = Request {
        codes: "wqj".into(),
        locked: vec![Segment {
            code: "wq".into(),
            text: "你".into(),
        }],
    };
    assert_eq!(decoder().decode(r).unwrap().candidates[0].text, "你是");
}
#[test]
fn invalid_or_oversized_input_does_not_panic() {
    for codes in ["你是谁".into(), "wq|j".into(), "a".repeat(97)] {
        assert!(
            decoder()
                .decode(Request {
                    codes,
                    ..Default::default()
                })
                .is_err()
        );
    }
    assert!(
        decoder()
            .decode(Request::default())
            .unwrap()
            .candidates
            .is_empty()
    );
}
#[test]
fn every_result_consumes_only_exact_code_table_edges() {
    let r = decoder()
        .decode(Request {
            codes: "wqjwqj".into(),
            ..Default::default()
        })
        .unwrap();
    for c in r.candidates {
        assert_eq!(
            c.segments
                .iter()
                .map(|s| s.code.as_str())
                .collect::<String>()
                + &c.pending,
            "wqjwqj"
        );
    }
}
