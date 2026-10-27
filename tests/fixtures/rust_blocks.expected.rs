fn blocks(items: &[u32]) {
    let total = 0;

    for item in items {
        consume(item);
    }

    let after = 1;

    if after > 0 {
        consume(after);
    } else if after == 0 {
        consume(0);
    }

    match after {
        0 => {}
        _ => {}
    }

    'outer: loop {
        break 'outer;
    }

    unsafe {
        call();
    }

    while total < 3 {
        consume(total);
    }

    consume(total);
}
