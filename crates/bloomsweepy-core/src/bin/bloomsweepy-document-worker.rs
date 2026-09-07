//! This allocator is installed only in the disposable document worker, never the GUI.
//! It caps live Rust allocation requests; OS mappings/allocator overhead are not
//! included, so this is not a claim of an exact whole-process RSS limit.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

const HEAP_BUDGET: usize = 128 * 1024 * 1024;
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
struct BudgetAllocator;

fn reserve(bytes: usize) -> bool {
    LIVE_BYTES
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |live| {
            live.checked_add(bytes).filter(|next| *next <= HEAP_BUDGET)
        })
        .is_ok()
}

// SAFETY: successful operations delegate to System using the original layout;
// accounting is atomic, includes realloc deltas, and is rolled back on failure.
unsafe impl GlobalAlloc for BudgetAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc(layout) };
        if pointer.is_null() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if !reserve(layout.size()) {
            return std::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if pointer.is_null() {
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe {
            System.dealloc(pointer, layout);
        }
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let growth = size.saturating_sub(layout.size());
        if !reserve(growth) {
            return std::ptr::null_mut();
        }
        let next = unsafe { System.realloc(pointer, layout, size) };
        if next.is_null() {
            LIVE_BYTES.fetch_sub(growth, Ordering::Relaxed);
        } else if size < layout.size() {
            LIVE_BYTES.fetch_sub(layout.size() - size, Ordering::Relaxed);
        }
        next
    }
}

#[global_allocator]
static ALLOCATOR: BudgetAllocator = BudgetAllocator;

fn main() {
    // A small diagnostic verifies rejection without ever allocating 128 MiB.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--check-heap-budget")
    {
        let layout = Layout::from_size_align(HEAP_BUDGET + 1, 1).expect("valid test layout");
        // Exercise the budget implementation directly. The global allocation
        // intrinsic may elide an unused alloc/dealloc pair or assume success in
        // optimized builds; that is not a reliable rejection diagnostic.
        let pointer = unsafe { ALLOCATOR.alloc(layout) };
        if !pointer.is_null() {
            unsafe {
                ALLOCATOR.dealloc(pointer, layout);
            }
            std::process::exit(1);
        }
        println!("heap-budget-rejection-ok");
        return;
    }
    std::process::exit(bloomsweepy_core::run_document_worker());
}
