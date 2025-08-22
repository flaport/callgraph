"""Write GDS with sample errors."""

from __future__ import annotations

import gdsfactory as gf
from cspdk.si220.cband import LAYER
from gdsfactory.component import Component
from gdsfactory.typings import LayerSpec

layer = LAYER.WG
layer1 = LAYER.WG


@gf.cell
def nxn(
    west: int = 1,
    east: int = 4,
    north: int = 0,
    south: int = 0,
    xsize: float = 8.0,
    ysize: float = 8.0,
    wg_width: float = 0.45,
    layer: LayerSpec = "WG",
    wg_margin: float = 1.0,
) -> Component:
    """Returns nxn component.

    Args:
        west: number of waveguides on the west side.
        east: number of waveguides on the east side.
        north: number of waveguides on the north side.
        south: number of waveguides on the south side.
        xsize: size of the component in x direction.
        ysize: size of the component in y direction.
        wg_width: waveguide width.
        layer: layer of the waveguide.
        wg_margin: margin between waveguides.
    """
    return gf.c.nxn(
        west=west,
        east=east,
        north=north,
        south=south,
        xsize=xsize,
        ysize=ysize,
        wg_width=wg_width,
        layer=layer,
        wg_margin=wg_margin,
    )


if __name__ == "__main__":
    c = nxn()
    c.show()
