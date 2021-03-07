//! Reactive signals.

use std::cell::RefCell;
use std::rc::Rc;

pub struct Signal<T> {
    inner: Rc<T>,
    observers: Vec<Rc<Computation>>,
}

impl<T> Signal<T> {
    fn new(value: T) -> Self {
        Self {
            inner: Rc::new(value),
            observers: Vec::new(),
        }
    }

    fn observe(&mut self, handler: Rc<Computation>) {
        self.observers.push(handler);
    }

    fn update(&mut self, new_value: T) {
        self.inner = Rc::new(new_value);
    }

    fn trigger_observers(&self) {
        for observer in &self.observers {
            observer.0();
        }
    }
}

/// A derived computation from a signal. Takes the new value as an input.
pub struct Computation(Box<dyn Fn()>);

thread_local! {
    static HANDLER: RefCell<Option<Rc<Computation>>> = RefCell::new(None);

    /// To add the dependencies, iterate through functions and execute them.
    static DEPENDENCIES: RefCell<Option<Vec<Box<dyn Fn()>>>> = RefCell::new(None);
}

pub fn create_signal<T: 'static>(value: T) -> (Rc<impl Fn() -> Rc<T>>, Rc<impl Fn(T)>) {
    let signal = Rc::new(RefCell::new(Signal::new(value)));

    let getter = {
        let signal = signal.clone();
        move || {
            // if inside an effect, add this signal to dependency list
            DEPENDENCIES.with(|dependencies| {
                if dependencies.borrow().is_some() {
                    let signal = signal.clone();
                    let handler =
                        HANDLER.with(|handler| handler.borrow().as_ref().unwrap().clone());

                    dependencies
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .push(Box::new(move || {
                            signal.borrow_mut().observe(handler.clone())
                        }));
                }
            });

            signal.borrow().inner.clone()
        }
    };

    let setter = {
        let signal = signal.clone();
        move |new_value| {
            match signal.try_borrow_mut() {
                Ok(mut signal) => signal.update(new_value),
                // If the signal is already borrowed, that means it is borrowed in the getter, thus creating a cyclic dependency.
                Err(_err) => panic!("cannot create cyclic dependency"),
            };
            signal.borrow().trigger_observers();
        }
    };

    (Rc::new(getter), Rc::new(setter))
}

pub fn create_effect<F>(effect: F)
where
    F: Fn() + 'static,
{
    DEPENDENCIES.with(|dependencies| {
        if dependencies.borrow().is_some() {
            unimplemented!("nested dependencies are not supported")
        }

        let effect = Rc::new(Computation(Box::new(effect)));

        *dependencies.borrow_mut() = Some(Vec::new());
        HANDLER.with(|handler| *handler.borrow_mut() = Some(effect.clone()));

        // run effect for the first time to attach all the dependencies
        effect.0();

        // attach dependencies
        for dependency in dependencies.borrow().as_ref().unwrap() {
            dependency();
        }

        // Reset dependencies for next effect hook
        *dependencies.borrow_mut() = None;
    })
}

pub fn create_memo<'out, F, Out>(derived: F) -> impl Fn() -> Rc<Out>
where
    F: Fn() -> Out + 'static,
    Out: 'static,
{
    let derived = Rc::new(derived);
    let memo = Rc::new(RefCell::new(None));

    create_effect({
        let derived = derived.clone();
        let memo = memo.clone();
        move || {
            *memo.borrow_mut() = Some(Rc::new(derived()));
        }
    });

    // return memoized result
    move || memo.borrow().as_ref().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn signals() {
        let (state, set_state) = create_signal(0);
        assert_eq!(*state(), 0);

        set_state(1);
        assert_eq!(*state(), 1);
    }

    #[test]
    fn effects() {
        let (state, set_state) = create_signal(0);

        let (double, set_double) = create_signal(-1);

        create_effect({
            let set_double = set_double.clone();
            move || {
                set_double(*state() * 2);
            }
        });
        assert_eq!(*double(), 0); // calling create_effect should call the effect at least once

        set_state(1);
        assert_eq!(*double(), 2);
        set_state(2);
        assert_eq!(*double(), 4);
    }

    #[test]
    fn memo() {
        let (state, set_state) = create_signal(0);

        let double = create_memo(move || *state() * 2);
        assert_eq!(*double(), 0);

        set_state(1);
        assert_eq!(*double(), 2);

        set_state(2);
        assert_eq!(*double(), 4);
    }

    #[test]
    /// Make sure value is memoized rather than executed on demand.
    fn memo_only_run_once() {
        let (state, set_state) = create_signal(0);

        // use a Cell instead of a signal to prevent circular dependencies
        // TODO: change to create_signal once explicit tracking is implemented
        let counter = Rc::new(Cell::new(0));

        let double = create_memo({
            let counter = counter.clone();
            move || {
                counter.set(counter.get() + 1);

                *state() * 2
            }
        });
        assert_eq!(counter.get(), 1); // once for calculating initial derived state

        set_state(2);
        assert_eq!(counter.get(), 2);
        assert_eq!(*double(), 4);
        assert_eq!(counter.get(), 2); // should still be 2 after access
    }
}