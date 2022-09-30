use dioxus::prelude::*;

pub fn launch_ui() {
    dioxus_tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let radius = 0;

    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            justify_content: "center",
            align_items: "center",
            background_color: "hsl(248, 53%, 58%)",
            border_style: "solid solid solid solid",
            border_width: "thick",
            border_radius: "{radius}px",
            border_color: "#FFFFFF #FFFFFF #FFFFFF #FFFFFF",

            "{radius}"
        }
    })
}
