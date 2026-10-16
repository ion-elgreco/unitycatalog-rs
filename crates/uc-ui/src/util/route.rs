use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// The route's parameters `names`, as one value that changes only when one
/// of them does. A page builds its content again from them: the router
/// keeps the page when only the parameters change, such as from one table
/// to another.
pub fn route_params<const N: usize>(names: [&'static str; N]) -> Memo<[String; N]> {
    let params = use_params_map();
    Memo::new(move |_| params.with(|params| names.map(|name| params.get(name).unwrap_or_default())))
}
