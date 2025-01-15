use axum::Router;
pub mod rpm;
pub mod tag;
pub fn route(router: Router) -> Router {
    macro_rules! apply_routes {
        ($router:expr, [$($module:ident),*]) => {{
            let mut router = $router;
            $(
                router = $module::route(router);
            )*
            router
        }};
    }

    apply_routes!(router, [rpm, tag])
}
