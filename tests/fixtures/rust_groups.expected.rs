fn groups(mut a: u32) {
    let x = 1;
    let y = 2;
    a = x;

    a += y;
    a -= 1;
    call(a);
    other(a)?;
    task.await;
    println!("{a}");

    let z = match a {
        0 => 1,
        _ => 2,
    };

    let w = 3;
    // a paragraph the author meant

    let v = 4;
    drop((v, w, z));
}
