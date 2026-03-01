use dioxus::prelude::*;

#[component]
pub fn Pagination(current_page: Signal<usize>, total_pages: usize, total_items: usize, page_size: usize) -> Element {
    let page = current_page();
    let start = page * page_size + 1;
    let end = ((page + 1) * page_size).min(total_items);

    rsx! {
        div { class: "flex justify-between items-center py-3 px-4 border-b border-gray-700 bg-gray-900",
            p { class: "text-sm text-gray-400",
                "Showing {start}–{end} of {total_items}"
            }
            div { class: "flex items-center space-x-2",
                button {
                    class: "px-3 py-1 rounded text-sm bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-150",
                    disabled: page == 0,
                    onclick: move |_| current_page.set(page.saturating_sub(1)),
                    "Previous"
                }
                span { class: "px-3 py-1 text-sm text-gray-400",
                    "Page {page + 1} of {total_pages}"
                }
                button {
                    class: "px-3 py-1 rounded text-sm bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-150",
                    disabled: page + 1 >= total_pages,
                    onclick: move |_| current_page.set(page + 1),
                    "Next"
                }
            }
        }
    }
}
