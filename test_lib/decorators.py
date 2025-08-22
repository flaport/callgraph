import functools
from typing import Any

def my_decorator(func):
    """A simple decorator."""
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        print(f"Calling {func.__name__}")
        return func(*args, **kwargs)
    return wrapper

def timer(func):
    """Timer decorator."""
    def wrapper(*args, **kwargs):
        import time
        start = time.time()
        result = func(*args, **kwargs)
        end = time.time()
        print(f"{func.__name__} took {end - start} seconds")
        return result
    return wrapper

@my_decorator
def simple_function():
    """A function with a single decorator."""
    return "Hello"

@my_decorator
@timer
def complex_function(x, y):
    """A function with multiple decorators."""
    result = x + y
    return result

@functools.lru_cache(maxsize=128)
def cached_fibonacci(n):
    """Fibonacci with caching decorator."""
    if n <= 1:
        return n
    return cached_fibonacci(n - 1) + cached_fibonacci(n - 2)

class ExampleClass:
    """Class with decorated methods."""
    
    @staticmethod
    def static_method():
        """A static method."""
        return "static"
    
    @classmethod
    def class_method(cls):
        """A class method."""
        return cls.__name__
    
    @property
    def some_property(self):
        """A property."""
        return "property value"
    
    @my_decorator
    def decorated_method(self):
        """A method with custom decorator."""
        return self.some_property
    
    @staticmethod
    @timer
    def multi_decorated_static():
        """Static method with multiple decorators."""
        return "timed static"
