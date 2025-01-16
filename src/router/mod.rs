use axum::Router;
pub mod gpg_keys;
pub mod rpm;
pub mod tag;
macro_rules! apply_routes {
    ([$($module:ident),*]) => {
        pub fn route(mut router: Router) -> Router {
            $(
                router = router.merge($module::route());
            )*
            router
        }
    };
}

apply_routes!([rpm, tag]);
