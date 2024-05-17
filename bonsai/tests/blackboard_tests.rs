#![allow(dead_code, unused_imports, unused_variables)]
use std::collections::HashMap;

use bonsai_bt::{Action, Event, Sequence, Success, UpdateArgs, Period, BT};

use crate::blackboard_tests::TestActions::{Dec, Inc};

/// Some test actions.
#[derive(Clone, Debug, Copy)]
pub enum TestActions {
    /// Increment accumulator.
    Inc,
    /// Decrement accumulator.
    Dec,
}

// A test state machine that can increment and decrement.
fn tick(mut acc: i32, dt: f64, bt: &mut BT<TestActions, HashMap<String, i32>>) -> i32 {
    let e: Event = UpdateArgs { dt }.into();

    let (_s, _t) = bt.tick(&e, &mut |args, _| match *args.action {
        Inc => {
            acc += 1;
            (Success, args.dt)
        }
        Dec => {
            acc -= 1;
            (Success, args.dt)
        }
    });

    // update counter in blackboard
    let bb = bt.get_blackboard_mut();

    bb.get_db_mut()
        .entry("count".to_string())
        .and_modify(|count| *count = acc)
        .or_insert(0)
        .to_owned()
}

#[test]
fn test_crate_bt() {
    let a: i32 = 0;
    let seq = Sequence(vec![
        Period(1.0),
        Action(Inc),
        Period(1.0),
        Action(Inc),
        Period(0.5),
        Action(Dec),
    ]);

    let h: HashMap<String, i32> = HashMap::new();
    let mut bt = BT::new(seq, h);
    let a = tick(a, 0.5, &mut bt);
    assert_eq!(a, 0);
    let a = tick(a, 0.5, &mut bt);
    assert_eq!(a, 1);
    let a = tick(a, 0.5, &mut bt);
    assert_eq!(a, 1);
    let a = tick(a, 0.5, &mut bt);
    assert_eq!(a, 2);
    let a = tick(a, 0.5, &mut bt);
    assert_eq!(a, 1);

    let bb = bt.get_blackboard_mut();
    let count = bb.get_db_mut().get("count").unwrap();
    assert_eq!(*count, 1);
}
