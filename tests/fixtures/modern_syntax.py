type Point = tuple[float, float]
type Pair[T] = tuple[T, T]
origin = Point
if origin:
    pass


class Box[T]:
    def get[U](self, default: U) -> T | U:
        value = self.value
        if value is None:
            return default
        return value


def quotes(row):
    key = f"{row["key"]}"
    joined = f"{"\n".join(row)}"
    wide = f"""{
        row

    }"""
    template = t"{row}"
    if key:
        pass
    return joined, wide, template


def groups(run):
    try:
        run()
    except* ValueError:
        first = 1
        second = 2

        third = 3
    try:
        run()
    except KeyError, IndexError:
        pass
    return first, second, third


async def stream(source):
    total = 0
    async for item in source:
        total += item
    async with source:
        pass
    await source.close()
    return total


def walrus(items):
    if (count := len(items)) > 1:
        pass
    return count


def delegate(inner):
    first = 1
    second = 2
    yield from inner


def matching(command):
    match command:
        case [name, *_] if name:
            found = name
            if found:
                pass
        case _:
            pass
