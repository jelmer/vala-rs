//! Concurrent compilation against libvala's process-global context stack, which
//! corrupted that stack (and crashed) before `with_current` serialised access.

use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use vala::{CodeContext, Parser, Profile, SourceFile, SourceFileType};

/// Compile a file with exactly one deliberate type error, returning the number
/// of errors this context saw. A source that compiled cleanly would make a poor
/// oracle: with no diagnostics to misattribute, a corrupted stack looks just
/// like a healthy one.
fn compile_with_one_error(n: usize) -> i32 {
    let src = format!("class C{n} {{\n    public int x{n} = \"not an int\";\n}}\n");
    let ctx = CodeContext::new();
    ctx.with_current(|ctx| {
        // Without a profile even `int` is undefined, and the check would fail
        // for reasons of its own.
        ctx.set_target_profile(Profile::GObject, true);
        let file = SourceFile::new(
            ctx,
            SourceFileType::Source,
            &format!("t{n}.vala"),
            Some(&src),
        );
        ctx.add_source_file(&file);
        Parser::new().parse(ctx);
        ctx.check();
        ctx.report().errors()
    })
}

#[test]
fn each_thread_sees_only_its_own_diagnostics() {
    let handles: Vec<_> = (0..16)
        .map(|n| thread::spawn(move || compile_with_one_error(n)))
        .collect();
    for handle in handles {
        assert_eq!(
            handle.join().expect("thread panicked"),
            1,
            "a thread saw a diagnostic count that is not its own"
        );
    }
}

#[test]
fn repeated_concurrent_rounds() {
    // One round is not always enough to trip the race; several make it reliable.
    for _ in 0..8 {
        let handles: Vec<_> = (0..8)
            .map(|n| thread::spawn(move || compile_with_one_error(n)))
            .collect();
        for handle in handles {
            assert_eq!(handle.join().expect("thread panicked"), 1);
        }
    }
}

#[test]
fn compilations_never_overlap() {
    // Measure the guarantee directly rather than inferring it from symptoms.
    static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
    static MAX_SEEN: AtomicUsize = AtomicUsize::new(0);

    let handles: Vec<_> = (0..12)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..10 {
                    let ctx = CodeContext::new();
                    ctx.with_current(|_| {
                        let n = IN_FLIGHT.fetch_add(1, Ordering::SeqCst) + 1;
                        MAX_SEEN.fetch_max(n, Ordering::SeqCst);
                        thread::yield_now();
                        IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("thread panicked");
    }

    assert_eq!(
        MAX_SEEN.load(Ordering::SeqCst),
        1,
        "two threads were inside with_current at the same time"
    );
}

#[test]
#[should_panic(expected = "not re-entrant")]
fn nested_with_current_panics_rather_than_deadlocking() {
    let ctx = CodeContext::new();
    ctx.with_current(|_| {
        let inner = CodeContext::new();
        inner.with_current(|_| ());
    });
}

#[test]
fn a_panic_does_not_wedge_the_thread() {
    // The guard must clear the re-entrancy flag on unwind, or every later call
    // on this thread would wrongly panic as if nested.
    let ctx = CodeContext::new();
    let caught = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        ctx.with_current(|_| panic!("boom"));
    }));
    assert!(caught.is_err(), "the panic should propagate");

    let ctx = CodeContext::new();
    assert_eq!(ctx.with_current(|_| 42), 42, "thread left wedged");
    assert_eq!(ctx.with_current(|_| 7), 7, "the flag is sticky");
}

#[test]
fn a_panic_does_not_hold_the_lock_forever() {
    // Poisoning is recovered, so one panicking compilation must not starve the
    // rest.
    thread::spawn(|| {
        let ctx = CodeContext::new();
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            ctx.with_current(|_| panic!("boom"));
        }));
    })
    .join()
    .expect("the panic is caught inside the thread");

    let other = thread::spawn(|| {
        let ctx = CodeContext::new();
        ctx.with_current(|_| 1)
    });
    assert_eq!(other.join().unwrap(), 1, "lock never released");
}

#[test]
fn a_reentrancy_panic_leaves_the_outer_call_intact() {
    // The re-entrancy check fires before the lock is taken, so the outer call
    // still owns it.
    let ctx = CodeContext::new();
    let outer = ctx.with_current(|_| {
        let nested = CodeContext::new();
        let caught = panic::catch_unwind(panic::AssertUnwindSafe(|| nested.with_current(|_| 0)));
        assert!(caught.is_err(), "the nested call must panic");
        99
    });
    assert_eq!(outer, 99);
    assert_eq!(ctx.with_current(|_| 5), 5, "outer call left state behind");
}

#[test]
fn repeated_panics_do_not_accumulate() {
    let ctx = CodeContext::new();
    for _ in 0..3 {
        let caught = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            ctx.with_current(|_| -> i32 { panic!("boom") })
        }));
        assert!(caught.is_err());
    }
    assert_eq!(
        ctx.with_current(|_| 1),
        1,
        "state accumulated across panics"
    );
}
