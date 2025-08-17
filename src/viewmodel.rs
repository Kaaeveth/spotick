#[macro_export]
macro_rules! callback {
    ($prop:ident, |$app_ref:ident $(,)? $( $params:ident ),*| $handler:block) => {{
        $app_ref.$prop({
            let app_weak = $app_ref.as_weak();
            move |$( $params ),*| {
                let $app_ref = app_weak.unwrap();
                $handler
            }
        });
    }};
}
