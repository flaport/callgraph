import gdsfactory as gf


@gf.cell
def spiral_delay(length=200) -> gf.Component:
    c = gf.components.spiral(length=length)
    return c
