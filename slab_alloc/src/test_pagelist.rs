use super::*;
use std::ptr::eq;

#[test]
fn test_pagelist_assign_singleton() {
    let mut list: PageList = PageList::new();
    let mut link = PageLink::null();
    list.assign_singleton(&mut link);
    assert!(eq(list.head, &mut link) && eq(list.tail, &mut link), "Singleton assignment failed");
    assert!(link.next.is_null() && link.prev.is_null(), "Singleton link's next and prev should be null");
}

#[test]
fn test_pagelist_is_empty_after_new() {
    let list: PageList = PageList::new();
    assert!(list.head.is_null() && list.tail.is_null(), "New list should be empty");
}

#[test]
fn test_pagelist_push_back_single() {
    let mut list = PageList::new();
    let mut link = PageLink::null();
    list.push_back(&mut link);
    assert!(eq(list.head, &mut link), "Head should point to pushed link");
    assert!(eq(list.tail, &mut link), "Tail should point to pushed link");
    assert!(link.next.is_null() && link.prev.is_null(), "Pushed link's next and prev should be null");
}

#[test]
fn test_pagelist_push_back_multiple() {
    let mut list = PageList::new();
    let mut link1 = PageLink::null();
    let mut link2 = PageLink::null();
    list.push_back(&mut link1);
    list.push_back(&mut link2);
    assert!(eq(list.head, &mut link1), "Head should be first link");
    assert!(eq(list.tail, &mut link2), "Tail should be last link");
    assert!(eq(link1.next, &mut link2), "First link's next should be second link");
    assert!(link1.prev.is_null(), "First link's prev should be null");
    assert!(link2.next.is_null(), "Second link's next should be null");
    assert!(eq(link2.prev, &mut link1), "Second link's prev should be first link");
}

#[test]
fn test_pagelist_remove_only_element() {
    let mut list = PageList::new();
    let mut link = PageLink::null();
    list.push_back(&mut link);
    unsafe { list.remove(&mut link); }
    assert!(list.head.is_null() && list.tail.is_null(), "List should be empty after removing only element");
    assert!(link.next.is_null() && link.prev.is_null(), "Removed link's next and prev should be null");
}

#[test]
fn test_pagelist_remove_head() {
    let mut list = PageList::new();
    let mut link1 = PageLink::null();
    let mut link2 = PageLink::null();
    list.push_back(&mut link1);
    list.push_back(&mut link2);
    unsafe { list.remove(&mut link1); }
    assert!(eq(list.head, &mut link2), "Head should be updated to second link");
    assert!(eq(list.tail, &mut link2), "Tail should remain second link");
    assert!(link2.prev.is_null() && link2.next.is_null(), "New head's link should be null");
    assert!(link1.prev.is_null() && link1.next.is_null(), "Removed link should be null");
}

#[test]
fn test_pagelist_remove_tail() {
    let mut list = PageList::new();
    let mut link1 = PageLink::null();
    let mut link2 = PageLink::null();
    list.push_back(&mut link1);
    list.push_back(&mut link2);
    unsafe { list.remove(&mut link2); }
    assert!(eq(list.head, &mut link1), "Head should remain first link");
    assert!(eq(list.tail, &mut link1), "Tail should be updated to first link");
    assert!(link2.prev.is_null() && link2.next.is_null(), "New tail's link should be null");
    assert!(link1.prev.is_null() && link1.next.is_null(), "Removed link should be null");
}

#[test]
fn test_pagelist_remove_middle() {
    let mut list = PageList::new();
    let mut link1 = PageLink::null();
    let mut link2 = PageLink::null();
    let mut link3 = PageLink::null();
    list.push_back(&mut link1);
    list.push_back(&mut link2);
    list.push_back(&mut link3);
    unsafe { list.remove(&mut link2); }
    assert!(eq(list.head, &mut link1), "Head should remain first link");
    assert!(eq(list.tail, &mut link3), "Tail should remain last link");
    assert!(eq(link1.next, &mut link3), "First link's next should be third link");
    assert!(eq(link3.prev, &mut link1), "Third link's prev should be first link");
    assert!(link2.next.is_null() && link2.prev.is_null(), "Removed link's next and prev should be null");
}

#[test]
fn test_pagelist_iterate_forward() {
    let mut list = PageList::new();
    let mut link1 = PageLink::null();
    let mut link2 = PageLink::null();
    let mut link3 = PageLink::null();
    list.push_back(&mut link1);
    list.push_back(&mut link2);
    list.push_back(&mut link3);

    let mut current = list.head;
    let mut count = 0;
    while !current.is_null() {
        count += 1;
        unsafe { current = (*current).next; }
    }
    assert_eq!(count, 3, "Should iterate over 3 elements");
}

#[test]
fn test_pagelist_iterate_backward() {
    let mut list = PageList::new();
    let mut link1 = PageLink::null();
    let mut link2 = PageLink::null();
    let mut link3 = PageLink::null();
    list.push_back(&mut link1);
    list.push_back(&mut link2);
    list.push_back(&mut link3);

    let mut current = list.tail;
    let mut count = 0;
    while !current.is_null() {
        count += 1;
        unsafe { current = (*current).prev; }
    }
    assert_eq!(count, 3, "Should iterate backward over 3 elements");
}
