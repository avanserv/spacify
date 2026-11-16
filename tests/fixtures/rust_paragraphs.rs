fn paragraphs(mut a: u32) {
    let x = 1;
    let y = 2;


    a = x;

    a += y;
    call(a);


    let z = compute(
        a,
    );
    // a paragraph the author meant


    let w = 3;
    drop((w, z));
}
