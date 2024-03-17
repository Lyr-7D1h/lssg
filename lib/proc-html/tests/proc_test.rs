use proc_html::html;
use std::collections::HashMap;
use virtual_dom::Html;

/// Utility function to convert iteratables into attributes hashmap
pub fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
    arr: I,
) -> HashMap<String, String> {
    arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
}

fn text(text: &str) -> Html {
    Html::Text { text: text.into() }
}

fn p(children: Vec<Html>) -> Html {
    Html::Element {
        tag: "p".into(),
        attributes: HashMap::new(),
        children,
    }
}

#[test]
fn static_html_works() {
    let input = html! {
        <a href="test.com">
            <i class="fa-solid fa-rss"></i>Test</a>
        <button disabled></button>
    };

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

    let expected = Html::Element {
        tag: "div".into(),
        attributes: HashMap::new(),
        children: vec![Html::Element {
            tag: "a".into(),
            attributes: to_attributes([("href", "link.com")]),
            children: vec![Html::Text {
                text: "[other](other.com)".into(),
            }],
        }],
    };
    assert_eq!(expected, input);
}

#[test]
fn html_interpolation_works() {
    let title = "Lyr";
    let paragraph = "Story of an awesome dev";
    let class = "test";
    let input = html!(<h1 class="{class}">{title}</h1><p>{paragraph}</p>);
    assert_eq!(
        vec![
            Html::Element {
                tag: "h1".into(),
                attributes: to_attributes([("class", "test")]),
                children: vec![text("Lyr")]
            },
            p(vec![text("Story of an awesome dev")])
        ],
        input
    )
}
