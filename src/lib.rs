#![deny(missing_docs)]
//! The Ruukh framework

extern crate wasm_bindgen;
#[cfg(test)]
extern crate wasm_bindgen_test;
extern crate indexmap;

use component::{Render, RootParent};
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use vdom::vcomponent::{ComponentManager, ComponentWrapper};
use wasm_bindgen::prelude::*;
#[cfg(test)]
use wasm_bindgen_test::*;
use web_api::*;

#[cfg(test)]
wasm_bindgen_test_configure!(run_in_browser);

mod component;
mod dom;
pub mod vdom;
pub mod web_api;

#[allow(missing_docs)]
pub mod prelude {
    pub use component::{Component, EventsPair, Lifecycle, Render, Status};
    pub use vdom::{
        vcomponent::VComponent,
        velement::{Attribute, EventListener, VElement},
        vlist::VList,
        vtext::VText,
        {Key, VNode},
    };
}

/// The main entry point to use your root component and run it on the browser.
///
/// App is a simple wrapper on top of the ComponentWrapper itself with basic
/// requirements that it should not have any props and events.
pub struct App<COMP>
where
    COMP: Render<Props = (), Events = ()>,
{
    manager: ComponentWrapper<COMP, RootParent>,
}

impl<COMP> App<COMP>
where
    COMP: Render<Props = (), Events = ()>,
{
    /// The App constructor.
    ///
    /// # Example
    /// ```
    /// let my_app = App::<MyApp>::new();
    /// ```
    pub fn new() -> App<COMP> {
        App {
            manager: ComponentWrapper::new((), ()),
        }
    }

    /// Mount the app on the given element in the DOM.
    /// Be careful to return the `ReactiveApp` to the JS side because we want our
    /// app to live for 'static lifetimes (i.e. As long as the browser/tab runs).
    ///
    /// # Example
    /// ```
    /// #[wasm_bindgen]
    /// fn run() -> ReactiveApp {
    ///     App::<MyApp>::new().mount("app")
    /// }
    /// ```
    pub fn mount<E: AppMount>(mut self, element: E) -> ReactiveApp {
        let parent = element.app_mount();
        let (mut channel, sender) = ReactiveApp::new();

        // Every component requires a render context, so provided a void context.
        let root_parent = Shared::new(());

        // The first render
        self.manager
            .render_walk(parent.as_ref(), None, root_parent.clone(), sender.clone())
            .unwrap();

        // Rerender when it receives update messages.
        channel.on_message(move || {
            self.manager
                .render_walk(parent.as_ref(), None, root_parent.clone(), sender.clone())
                .unwrap();
        });

        channel
    }
}

/// The mounted app which reacts to any change event messaged by the components
/// in the tree.
///
/// It stores the receiver end of the message port which listens onto any messages
/// and invokes the app to update itself.
#[wasm_bindgen]
pub struct ReactiveApp {
    rx: MessagePort,
    on_message: Option<Closure<FnMut(JsValue)>>,
}

impl ReactiveApp {
    fn new() -> (ReactiveApp, MessageSender) {
        let msg_channel = MessageChannel::new();
        (
            ReactiveApp {
                rx: msg_channel.port2(),
                on_message: None,
            },
            MessageSender {
                tx: msg_channel.port1(),
            },
        )
    }

    /// When it receives a message, invoke the handler.
    fn on_message<F: FnMut() + 'static>(&mut self, mut handler: F) {
        let closure: Closure<FnMut(JsValue)> = Closure::wrap(Box::new(move |_| handler()));
        self.rx.on_message(&closure);
        self.on_message = Some(closure);
    }
}

/// Whenever the state changes in components, MessageSender is responsible
/// to message the App. The App reacts to the messages and updates it tree.   
#[derive(Clone)]
struct MessageSender {
    tx: MessagePort,
}

impl MessageSender {
    /// The components need to call this method, when it desires the app to
    /// rerender.
    fn do_react(&self) {
        self.tx
            .post_message(&JsValue::null())
            .expect("Could not send the message");
    }
}

/// A Shared Value
pub struct Shared<T>(Rc<RefCell<T>>);

impl<T> Shared<T> {
    fn new(val: T) -> Shared<T> {
        Shared(Rc::new(RefCell::new(val)))
    }

    /// Borrow the inner value.
    pub fn borrow(&self) -> Ref<T> {
        self.0.borrow()
    }

    /// Borrow the inner value mutably.
    pub fn borrow_mut(&self) -> RefMut<T> {
        self.0.borrow_mut()
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

/// Trait to get an element on which the App is going to mount.
pub trait AppMount {
    /// Consume `self` and get an element from the DOM.
    fn app_mount(self) -> Element;
}

impl<'a> AppMount for &'a str {
    fn app_mount(self) -> Element {
        html_document.get_element_by_id(self).expect(&format!(
            "Could not find element with id `{}` to mount the App.",
            self
        ))
    }
}

impl AppMount for Element {
    fn app_mount(self) -> Element {
        self
    }
}

impl AppMount for String {
    fn app_mount(self) -> Element {
        self.as_str().app_mount()
    }
}
