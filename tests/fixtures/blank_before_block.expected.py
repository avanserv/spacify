def f(items, flag):
    total = 0

    for item in items:
        total += item

    if flag:
        total = -total

    while total > 10:
        total -= 1

    with open("log") as handle:
        handle.write(str(total))

    try:
        pass
    except ValueError:
        pass
    async def g():
        pass


async def h(stream):
    count = 0

    async for chunk in stream:
        count += 1

    async with stream:
        pass
