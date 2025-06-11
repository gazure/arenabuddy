use leptos::prelude::*;

#[component]
pub fn MatchInfo(
    #[prop(into)] controller_player_name: Signal<String>,
    #[prop(into)] opponent_player_name: Signal<String>,
    #[prop(into)] did_controller_win: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow-md overflow-hidden">
            <div class="bg-gradient-to-r from-blue-500 to-blue-600 py-4 px-6">
                <h2 class="text-xl font-bold text-white">Match Information</h2>
            </div>
            <div class="p-6">
                <div class="mb-4">
                    <h3 class="text-lg font-semibold text-gray-700 mb-2">Players</h3>
                    <div class="flex flex-col gap-2">
                        <div class="bg-blue-50 p-3 rounded-md">
                            <span class="font-semibold">You</span>
                            {move || { format!(" {}", controller_player_name.get()) }}
                        </div>
                        <div class="bg-red-50 p-3 rounded-md">
                            <span class="font-semibold">Opponent</span>
                            {move || { format!(" {}", opponent_player_name.get()) }}
                        </div>
                    </div>
                </div>

                <div class="mb-4">
                    <h3 class="text-lg font-semibold text-gray-700 mb-2">Game Details</h3>
                    <div class="grid grid-cols-2 gap-2">
                        <div class="bg-gray-50 p-3 rounded-md">
                            <span class="text-sm text-gray-500 block">Format</span>
                            <span class="font-medium">unknown</span>
                        </div>
                        <div class="bg-gray-50 p-3 rounded-md">
                            <span class="text-sm text-gray-500 block">Result</span>
                            <span class="font-medium">
                                {move || {
                                    if did_controller_win.get() {
                                        view! {
                                            <span class="text-green-600 font-bold">Victory</span>
                                        }
                                            .into_view()
                                    } else {
                                        view! { <span class="text-red-600 font-bold">Defeat</span> }
                                            .into_view()
                                    }
                                }}
                            </span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
