import gdsfactory as gf
from cspdk.si220.cband import cells


@gf.cell
def mzi3(delta_length=30, mzi="mzi3"):
    # c = cells.mzi(delta_length=delta_length).dup()
    c = gf.get_component(mzi, delta_length=delta_length)
    c.flatten()
    return c


@gf.cell
def mzi4(delta_length=30, mzi="mzi3")::
    # c = cells.mzi(delta_length=delta_length).dup()
    c = gf.get_component(mzi, delta_length=delta_length)
    c.flatten()
    return c
