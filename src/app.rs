use leptos::*;
use leptos_meta::{provide_meta_context, Meta, Title};
use leptos_router::*;

use crate::{
    error_template::{AppError, ErrorTemplate},
    pages::mainpage::MainPage,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Meta name="charset" content="UTF-8"/>
        <Meta name="description" content="Leptosaurus"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <Meta name="theme-color" content="#e66956"/>

        //<Stylesheet id="leptos" href="/pkg/leptonic-template-ssr.css"/>
        //<Stylesheet href="https://fonts.googleapis.com/css?family=Roboto&display=swap"/>

        <Title text="Leptosaurus"/>

        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
        }>
            <Routes>
                <Route path="" view=|| view! { <MainPage/> }/>
            </Routes>
        </Router>
    }
}
