mod app;
mod components;
mod models;

use app::App;
use yew::Renderer;

fn main() {
    Renderer::<App>::new().render();
}
