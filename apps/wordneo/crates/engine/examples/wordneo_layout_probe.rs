//! Local, read-only geometry probe. Deliberately omits document text and image bytes.
use serde_json::{Value, json};
use wordcraft_engine::Session;
use wordcraft_layout::Placed;

fn items(items: &[Placed]) -> Vec<Value> {
    items.iter().filter_map(|item| match item {
        Placed::Lines { para, l0, l1, x, y, .. } => Some(json!({
            "x": x, "y": y, "height": para.height, "spacing": format!("{:?}", para.rp.line_spacing),
            "space_before": para.rp.space_before, "space_after": para.rp.space_after,
            "styles": para.styles.iter().map(|s| json!({"font":s.rc.font,"size":s.size,"ascent":s.ascent,"descent":s.descent})).collect::<Vec<_>>(),
            "lines": para.lines.iter().skip(*l0).take(l1.saturating_sub(*l0)).map(|l| json!({
                "top":l.top,"height":l.height,"baseline":l.baseline,"start":l.start,"stop":l.stop,
                "clusters":para.clusters.get(l.c0..l.c1).into_iter().flatten().map(|c| json!({
                    "kind":format!("{:?}",c.kind),"advance":c.adv,"height":c.obj_h,"descent":c.obj_d,"style":c.style,
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        })),
        Placed::Object { rect, .. } => Some(json!({"object_rect":[rect.x,rect.y,rect.w,rect.h]})),
        _ => None,
    }).collect()
}

fn main() -> Result<(), String> {
    let mut result = Vec::new();
    for path in std::env::args().skip(1) {
        let doc = wordcraft_engine::io::open_path(std::path::Path::new(&path))?;
        let mut session = Session::new(doc);
        let layout = session.layout();
        let first = layout.pages.first().map(|p| {
            json!({
                "body":[p.body.x,p.body.y,p.body.w,p.body.h],
                "header":items(&p.header),"footer":items(&p.footer),
                "body_first":items(&p.items).into_iter().take(3).collect::<Vec<_>>()
            })
        });
        result.push(json!({"file":path,"pages":layout.pages.len(),"first":first}));
    }
    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(())
}
