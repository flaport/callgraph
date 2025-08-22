import math

def calculate_area(radius):
    """Calculate the area of a circle."""
    return math.pi * square(radius)

def square(x):
    """Return the square of a number."""
    return x * x

def factorial(n):
    """Calculate factorial recursively."""
    if n <= 1:
        return 1
    return n * factorial(n - 1)

class MathUtils:
    """Utility class for mathematical operations."""
    
    @staticmethod
    def power(base, exponent):
        """Calculate power using math.pow."""
        return math.pow(base, exponent)
    
    @classmethod
    def fibonacci(cls, n):
        """Calculate fibonacci number."""
        if n <= 1:
            return n
        return cls.fibonacci(n - 1) + cls.fibonacci(n - 2)
    
    def distance(self, x1, y1, x2, y2):
        """Calculate distance between two points."""
        dx = x2 - x1
        dy = y2 - y1
        return math.sqrt(square(dx) + square(dy))
