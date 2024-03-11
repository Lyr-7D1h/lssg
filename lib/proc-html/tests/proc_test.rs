use std::collections::HashMap;

use proc_html::html;
use virtual_dom::Html;

/// Utility function to convert iteratables into attributes hashmap
pub fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
    arr: I,
) -> HashMap<String, String> {
    arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
}

#[test]
fn proc_html() {
    let input = html!(<a href="test.com">
            <i class="fa-solid fa-rss"></i>Test
        </a>
        <button disabled></button>
    );

    let expected = vec![
        Html::Element {
            tag: "a".into(),
            attributes: to_attributes([("href", "test.com")]),
            children: vec![
                Html::Element {
                    tag: "i".into(),
                    attributes: to_attributes([("class", "fa-solid fa-rss")]),
                    children: vec![],
                },
                Html::Text {
                    text: "Test".into(),
                },
            ],
        },
        Html::Text { text: "\n".into() },
        Html::Element {
            tag: "button".into(),
            attributes: to_attributes([("disabled", "")]),
            children: vec![],
        },
    ];

    assert_eq!(expected, input);

    let input = html!(<div>
    <a href="link.com">[other](other.com)</a>
    </div>);

    let expected = vec![Html::Element {
        tag: "div".into(),
        attributes: HashMap::new(),
        children: vec![
            Html::Text { text: "\n".into() },
            Html::Element {
                tag: "a".into(),
                attributes: to_attributes([("href", "link.com")]),
                children: vec![Html::Text {
                    text: "[other](other.com)".into(),
                }],
            },
            Html::Text { text: "\n".into() },
        ],
    }];
    assert_eq!(expected, input);
}
