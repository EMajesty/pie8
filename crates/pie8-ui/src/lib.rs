#![no_std]

pub mod adapter;
pub mod input;
pub mod model;
pub mod render;
pub mod ui;

pub use input::{Action, Button, ButtonEvent, EventKind, Input};
pub use model::Project;
pub use render::{HEIGHT, Palette, WIDTH, render};
pub use ui::{Controller, Screen};
