# This is a sample Python project for snapshot testing.

def hello_world():
    """Prints a greeting message."""
    print("Hello, world!")

class Greeter:
    """A simple class for greeting."""
    def __init__(self, name):
        self.name = name

    def greet(self):
        # This is a line comment inside a method
        message = f"Hello, {self.name}!"
        return message

if __name__ == "__main__":
    hello_world()
    greeter = Greeter("SnapshotUser")
    print(greeter.greet())
