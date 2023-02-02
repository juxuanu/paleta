# window.py
#
# Copyright 2023 nate-xyz
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-or-later

from gi.repository import Adw, Gtk, Gdk

# from paleta.pages.image_drop import ImageDropPage
# from paleta.pages.palettes import PalettePage
from .pages import ImageDropPage, PalettePage

import os
import html

image_mime_types = ['image/jpeg', 'image/png', 'image/tiff', 'image/webp']

@Gtk.Template(resource_path='/io/nxyz/Paleta/window.ui')
class Window(Adw.ApplicationWindow):
    __gtype_name__ = 'Window'
    
    toast_overlay = Gtk.Template.Child(name="toast_overlay")
    header_bar = Gtk.Template.Child(name="header_bar")
    view_switcher_title = Gtk.Template.Child(name="view-switcher-title")
    stack = Gtk.Template.Child(name="stack")
    open_image_button = Gtk.Template.Child(name="open_image_button")
    image_drop_page = Gtk.Template.Child(name="image_drop_page")
    palette_page = Gtk.Template.Child(name="palette_page")

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        print("Window")

        app = kwargs['application']

        self.db = app.db 
        self.db.window = self
        
        self.model = app.model
        
        self.saturate()
        
        if not self.db.try_loading_database():
            self.add_toast("Error loading database.")
    
        self.add_dialog()
       
        self.open_image_button.connect("clicked", self.show_open_dialog)
    
        self.clipboard = Gdk.Display.get_default().get_clipboard()
        self.setup_switcher_button()

    def saturate(self):
        self.image_drop_page.set_win(self)
        self.image_drop_page.set_db(self.db)

        self.palette_page.set_model(self.model)

    def setup_switcher_button(self):
        #adw switcher buttons
        squeezer = self.view_switcher_title.observe_children()[0]
        view_switcher = squeezer.observe_children()[0]
        self.switcher_buttons =  view_switcher.observe_children()
        #self.drop_button, self.palette_button = self.switcher_buttons
        for button in self.switcher_buttons:
            button.connect("clicked", self.switcher_button)

    def switcher_button(self, button):
        if self.check_switcher_title_bug(button):
            self.replace_switcher()

    def check_switcher_title_bug(self, active_button):
        error_string = 'button.flat.horizontal.toggle:active:dir(ltr)'
        for button in self.switcher_buttons:
            style_context = button.get_style_context()
            check_string = style_context.to_string(Gtk.StyleContextPrintFlags.SHOW_CHANGE).split(' ')[0]
            if button != active_button and error_string==check_string:
                return True
        return False
    
    def replace_switcher(self):        
        self.header_bar.remove(self.view_switcher_title)
        self.view_switcher_title = Adw.ViewSwitcherTitle()
        self.view_switcher_title.set_stack(self.stack)
        self.header_bar.set_title_widget(self.view_switcher_title)
        
        self.switcher_buttons =  self.view_switcher_title.observe_children()[0].observe_children()[0].observe_children()

        for button in self.switcher_buttons:
            button.connect("clicked", self.switcher_button)

    def add_dialog(self):
        self.folder_dialog = Gtk.FileChooserNative.new(title="Select an Image File", 
                                                        parent=self, 
                                                        action=Gtk.FileChooserAction.OPEN, 
                                                        accept_label="Open Image")

        f = Gtk.FileFilter()
        f.set_name(_("Image files"))
        for m in image_mime_types:
            f.add_mime_type(m)

        self.folder_dialog.connect("response", self.open_response)
        self.folder_dialog.add_filter(f)

    def show_open_dialog(self, _button):
        self.folder_dialog.show()

    def open_response(self, dialog, response):
        if response == Gtk.ResponseType.ACCEPT:
            image_uri = dialog.get_file().get_path()
            if self.image_drop_page.load_image(image_uri):
                self.stack.set_visible_child(self.image_drop_page)
                self.open_image_toast(image_uri)
            else:
                self.error_image_toast(image_uri)
        

    def error_image_toast(self, uri):
        base_name = os.path.basename(uri)
        self.add_toast("Error! Could not open image: {}".format(base_name))

    def open_image_toast(self, uri):
        base_name = os.path.basename(uri)
        self.add_toast("Opened image: {}".format(base_name))

    def add_toast(self, title: str, timeout: int = 1):
        toast = Adw.Toast.new(html.escape(title))
        toast.set_timeout(timeout)
        self.toast_overlay.add_toast(toast)

    def copy_color(self, hex_name):
        self.add_toast("Copied color {} to clipboard!".format(hex_name))
        self.clipboard.set(hex_name)
