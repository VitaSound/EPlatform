local Icons = {}

Icons.NAMES = {
    add = "img24:18",
    edit = "img24:19",
    delete = "img24:20",
    macro = "img24:85",
    refresh = "img24:8",
    view = "img24:15",
    ok = "img24:22",
    cancel = "img24:3"
}

function Icons.get(name_or_spec)
    return Icons.NAMES[name_or_spec] or name_or_spec
end

function Icons.set_button_icon(form, widget_name, name_or_spec)
    form:setIcon(widget_name, Icons.get(name_or_spec))
end

return Icons
