fn attributes() {
    let a = 1;
    #[cfg(test)]
    #[allow(unused)]
    let b = 2;
    #[rustfmt::skip]
    let   c = 3;
    if a > 0 {
        consume(a);
    }
    // belongs to the loop
    for i in 0..a {
        consume(i);
    }
    /*
     * belongs to the match
     */
    match a {
        _ => {}
    }
    let d = 4; /* trailing,
    spanning */
    if d > 0 {
        consume(d);
    }
    // spacify: off
    let e = 5;
    if e > 0 {
        consume(e);
    }
    // spacify: on
    let f = 6;
    if f > 0 {
        consume(f);
    }
}
