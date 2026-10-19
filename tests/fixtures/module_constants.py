import logging

logger = logging.getLogger(__name__)


TIMEOUT = 10

RETRIES: int = 3

LOW, HIGH = 0, 9
"""Bounds of a level."""

Http_Code = 200

handler = logging.StreamHandler()

level = logging.INFO


class Config:
    NAME = "config"

    path = "config.toml"


def f():
    LIMIT = 1

    count = 0


A = B = 1

C = 2

a = b = 1

D = 3

[E, F] = 4, 5

G, *REST = range(9)

Π = 3.14

ÉTÉ = "summer"

LIMIT: int

config.NAME = "app"

URL = "localhost"

if TYPE_CHECKING:
    H = 1

    i = 2

try:
    J = 1

    K = 2
except ImportError:
    J = 0
