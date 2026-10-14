def long_block(value):
    value = value + 1
    value = value * 2

    return value


def short_block(value):
    value = value + 1
    return value


def raises(value):
    check(value)
    log(value)

    raise ValueError(value)


def loop(values):
    for value in values:
        if value is None:
            continue

        handle(value)
        record(value)

        break


def generator(values):
    for value in values:
        result = transform(value)
        result = clean(result)

        yield result


def first(value):
    return value
