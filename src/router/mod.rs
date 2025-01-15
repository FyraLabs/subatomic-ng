use axum::Router;
pub mod rpm;
pub mod tag;
pub mod gpg_keys;
macro_rules! apply_routes {
    ([$($module:ident),*]) => {
        pub fn route(router: Router) -> Router {
            let mut router = router;
            $(
                router = $module::route(router);
            )*
            router
        }
    };
}

apply_routes!([rpm, tag]);
