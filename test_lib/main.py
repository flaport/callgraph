def hello_world():
    """A simple function that calls other functions."""
    greeting = get_greeting()
    name = get_name()
    print(f"{greeting}, {name}!")
    return process_result(greeting, name)

def get_greeting():
    """Returns a greeting."""
    return "Hello"

def get_name():
    """Returns a name."""
    return "World"

def process_result(greeting, name):
    """Processes the greeting and name."""
    result = f"{greeting} {name}"
    return result.upper()

class Calculator:
    """A simple calculator class."""
    
    def add(self, a, b):
        """Add two numbers."""
        return self._validate_and_add(a, b)
    
    def multiply(self, a, b):
        """Multiply two numbers."""
        result = self._validate_numbers(a, b)
        if result:
            return a * b
        return 0
    
    def _validate_numbers(self, a, b):
        """Validate that inputs are numbers."""
        return isinstance(a, (int, float)) and isinstance(b, (int, float))
    
    def _validate_and_add(self, a, b):
        """Validate and add numbers."""
        if self._validate_numbers(a, b):
            return a + b
        return 0

def main():
    """Main function."""
    hello_world()
    calc = Calculator()
    result1 = calc.add(5, 3)
    result2 = calc.multiply(4, 2)
    print(f"Results: {result1}, {result2}")

if __name__ == "__main__":
    main()
