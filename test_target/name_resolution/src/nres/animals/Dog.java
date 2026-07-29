package nres.animals;

public class Dog implements Pet {
    private final String name;

    public Dog(String name) {
        this.name = name;
    }

    @Override
    public String sound() {
        return "Woof";
    }

    @Override
    public String name() {
        return name;
    }
}
