fn load(path: &str) -> Result<Config> {
    let text = read(path)?;
    check(&text)?;
    save(&text)?;

    Ok(parse(text))
}

fn short(x: u32) -> u32 {
    let y = x + 1;
    y
}

fn early(items: &[u32]) -> Option<u32> {
    let first = items.first()?;
    let last = items.last()?;

    if first == last {
        return None;
    }

    let sum = first + last;
    let doubled = sum * 2;

    return Some(doubled);
}

fn values(items: &[u32]) -> u32 {
    let double = |x: u32| {
        let y = x * 2;
        let z = y + 1;
        z
    };
    let total = match items.len() {
        0 => 0,
        n => {
            let a = n as u32;
            let b = a * 2;
            b
        }
    };

    for item in items {
        let a = *item;
        let b = a + 1;
        let c = b + 2;

        continue;
    }

    total + double(1)
}
