use proc_virtual_dom::dom;
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
    let input = dom! {
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

    // convert to intermediate representation to compare
    let input: Vec<Html> = input.into_iter().map(|t| t.into()).collect();
    assert_eq!(expected, input);

    let input = dom!(<div>
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
    let input: Html = input.into();
    assert_eq!(expected, input);
}

#[test]
fn html_interpolation_works() {
    let title = "Lyr";
    let paragraph = "Story of an awesome dev";
    let class = "test";
    let input = dom!(<h1 class="{class}">{title}</h1><p>{paragraph}</p>);
    let expected = vec![
        Html::Element {
            tag: "h1".into(),
            attributes: to_attributes([("class", "test")]),
            children: vec![text("Lyr")],
        },
        p(vec![text("Story of an awesome dev")]),
    ];
    // convert to intermediate representation to compare
    let input: Vec<Html> = input.into_iter().map(|t| t.into()).collect();
    assert_eq!(expected, input);
}

#[test]
fn html_interpolating_html_works() {
    let html = dom!(<h1>Yay</h1>);
    let input = dom!(<div>{html}</div>);
    let expected = Html::Element {
        tag: "div".into(),
        attributes: HashMap::new(),
        children: vec![Html::Element {
            tag: "h1".into(),
            attributes: HashMap::new(),
            children: vec![text("Yay")],
        }],
    };

    assert_eq!(expected, input.into())
}

#[test]
fn void_meta_interpolations_works() {
    let title = "asdf";
    let input = dom!(<meta property="og:title" content="{title}"/>);
    let expected = Html::Element {
        tag: "meta".into(),
        attributes: to_attributes([("property", "og:title"), ("content", "asdf")]),
        children: vec![],
    };

    assert_eq!(expected, input.into())
}

#[test]
fn html_interpolation_works_for_self_closing() {
    let alt = String::from("Alt");
    let src = "Src";
    let input = dom!(<img src="{src}" alt="{alt}" />);
    let expected = Html::Element {
        tag: "img".into(),
        attributes: to_attributes([("src", "Src"), ("alt", "Alt")]),
        children: vec![],
    };
    assert_eq!(expected, input.into());
}

#[test]
fn html_with_js_works() {
    let input =
        dom!(<div class="default__carausel_item" onclick="default__toggleModal(event)"></div>);
    let expected = Html::Element {
        tag: "div".into(),
        attributes: to_attributes([
            ("class", "default__carausel_item"),
            ("onclick", "default__toggleModal(event)"),
        ]),
        children: vec![],
    };

    assert_eq!(expected, input.into())
}

#[test]
fn html_with_whitespace_and_interpolation() {
    let path = "/test";
    let name = "test";
    let children: Vec<virtual_dom::DomNode> = vec![virtual_dom::DomNode::create_text("child1")];

    // This has whitespace/newlines like in the actual code
    let input = dom!(
        <div class="default__sitetree_folder">
            <a href="{path}">{name}</a>
            <div class="default__sitetree_folder_content">
                {children}
            </div>
        </div>
    );

    // Should still return a single DomNode
    let _: virtual_dom::DomNode = input;
}
