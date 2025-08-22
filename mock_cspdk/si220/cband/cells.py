"""Mock CSPDK cells module for testing dependency resolution."""

def mzi(delta_length=10):
    """Mock MZI function."""
    return f"mzi with delta_length={delta_length}"

def ring_single(radius=10):
    """Mock single ring function."""
    return f"ring_single with radius={radius}"

def straight_heater_metal(length=100):
    """Mock straight heater with metal."""
    return f"straight_heater_metal with length={length}"

def die_with_pads():
    """Mock die with pads function."""
    return "die_with_pads"

def mzi_heater(delta_length=10):
    """Mock MZI with heater function."""
    return f"mzi_heater with delta_length={delta_length}"
