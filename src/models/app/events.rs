use crate::{
    hook::{self, Event},
    models::{App, Deployment},
};

#[must_use]
#[derive(Clone)]
pub enum AppEvent {
    Created(App),
    Deleting(App),
    Updated(App),
    Started(App, Deployment),
}

impl AppEvent {
    pub fn created(app: App) {
        AppEvent::Created(app).fire();
    }

    pub fn deleting(app: App) {
        AppEvent::Deleting(app).fire();
    }

    pub fn updated(app: App) {
        AppEvent::Updated(app).fire();
    }

    pub fn started(app: App, deployment: Deployment) {
        AppEvent::Started(app, deployment).fire();
    }
}

impl hook::Event for AppEvent {}
