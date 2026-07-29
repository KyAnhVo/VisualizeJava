package nres.app;

import nres.animals.Dog;
import nres.status.Priority;
import nres.store.Widget;
import nres.store.WidgetStore;
import nres.meta.Label;

public class Main {
    // Fully qualified, unimported reference -- exercises the project-index
    // fallback tier in resolve_qualified_name (scope miss, then
    // Project::get_origin_package hit).
    @Label("entry point")
    private nres.diamond.DiamondImpl marker;

    private final Dog dog;
    private final WidgetStore widgetStore;
    private Priority priority;

    public Main() {
        this.dog = new Dog("Rex");
        this.widgetStore = new WidgetStore();
        this.priority = Priority.MEDIUM;
    }

    public static void main(String[] args) {
        Main app = new Main();
        app.run();
    }

    private void run() {
        Widget widget = new Widget("w1", 10);
        widgetStore.save(widget);
    }
}
