import gdsfactory as gf
from cspdk.si220.cband import cells


@gf.cell
def mzi3(delta_length=30):
    c = cells.mzi(delta_length=delta_length).dup()
    c.flatten()
    return c
