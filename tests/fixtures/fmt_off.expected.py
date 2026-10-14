def f(value):
    value = 1
    # fmt: off
    if value:
        pass
    other = [1,2,
             3]
    # fmt: on
    if value:
        pass
    skipped = 1  # fmt: skip
    if value:
        pass
    # spacify: off
    a = 1

    b = 2
    # spacify: on
    c = 1
    d = 2
