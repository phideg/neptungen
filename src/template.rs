use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use config::Config;


fn load_template(name: &str, conf: &Config) -> Option<String> {
    conf.template_dir.as_ref().map(|template_dir| {
        let mut template = String::new();
        let mut path_buf = PathBuf::new();
        path_buf.push(template_dir);
        path_buf.push(name);
        match File::open(path_buf.as_path()).and_then(|mut f| f.read_to_string(&mut template)) {
            Ok(_) => template,
            Err(error) => {
                panic!("failed to open page template {}: {}",
                       path_buf.display(),
                       error)
            }
        }
    })
}

pub fn load_page_template(conf: &Config) -> String {
    load_template("page.liq", conf).unwrap_or_else(|| r#"
<!DOCTYPE html>
<html>
<head>
<title>{{title}}</title>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="https://www.w3schools.com/lib/w3.css">
<link rel="stylesheet" href="https://www.w3schools.com/lib/w3-theme-black.css">
<link rel="stylesheet" href="https://fonts.googleapis.com/css?family=Roboto">
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css">
<style>
html,body,h1,h2,h3,h4,h5,h6 {font-family: "Roboto", sans-serif}
.w3-sidenav a,.w3-sidenav h4 {padding: 12px;}
.w3-bar a {
    padding-top: 12px;
    padding-bottom: 12px;
}
</style>
</head>
<body>

    <!-- Navbar -->
    <div class="w3-top">
    <div class="w3-bar w3-theme w3-top w3-left-align w3-large">
        <a class="w3-bar-item w3-button w3-opennav w3-right w3-hide-large w3-hover-white w3-large w3-theme-l1" href="javascript:void(0)" onclick="w3_open()"><i class="fa fa-bars"></i></a>
        <a href="{{ root_dir }}index.html" class="w3-bar-item w3-button w3-theme-l1">Logo/Home</a>
    </div>
    </div>

    <!-- Sidenav -->
    <nav class="w3-sidenav w3-collapse w3-theme-l5 w3-animate-left" style="z-index:3;width:250px;margin-top:51px;" id="mySidenav">
    <a href="javascript:void(0)" onclick="w3_close()" class="w3-right w3-xlarge w3-padding-large w3-hover-black w3-hide-large" title="close menu">
        <i class="fa fa-remove"></i>
    </a>
    <h4><b>Menu</b></h4>
    {% for item in nav_items %}
      {% if item.menu_cmd == "OpenLevel" %}
        <a href="\#" class="w3-deep-orange">{{ item.name }}</a>
      {% else %}
        <a href="{{ root_dir }}{{ item.url }}" class="w3-hover-black">{{ item.name }}</a>
      {% endif %}
    {% endfor %}
    </nav>

    <!-- Overlay effect when opening sidenav on small screens -->
    <div class="w3-overlay w3-hide-large" onclick="w3_close()" style="cursor:pointer" title="close side menu" id="myOverlay"></div>

    <!-- Main content: shift it to the right by 250 pixels when the sidenav is visible -->
    <div class="w3-main" style="margin-left:250px">

    <div class="w3-row w3-padding-64">
        <div class="w3-twothird w3-container">
            {{ content }}
        </div>
    </div>

    <footer id="myFooter">
        <div class="w3-container w3-theme-l2 w3-padding-32">
        <h4>Generated with <a href="https://github.com/phideg/neptungen" target="_blank">neptungen</a></h4>
        </div>

        <div class="w3-container w3-theme-l1">
        <p>Powered by <a href="https://www.w3schools.com/w3css/default.asp" target="_blank">w3.css</a></p>
        </div>
    </footer>

    <!-- END MAIN -->
    </div>

    <script>
        // Get the Sidenav
        var mySidenav = document.getElementById("mySidenav");

        // Get the DIV with overlay effect
        var overlayBg = document.getElementById("myOverlay");

        // Toggle between showing and hiding the sidenav, and add overlay effect
        function w3_open() {
            if (mySidenav.style.display === 'block') {
                mySidenav.style.display = 'none';
                overlayBg.style.display = "none";
            } else {
                mySidenav.style.display = 'block';
                overlayBg.style.display = "block";
            }
        }

        // Close the sidenav with the close button
        function w3_close() {
            mySidenav.style.display = "none";
            overlayBg.style.display = "none";
        }
    </script>

</body>
</html>
    "#.to_owned())
}

pub fn load_gallery_template(conf: &Config) -> String {
    load_template("gallery.liq", conf).unwrap_or_else(|| r#"
<!DOCTYPE html>
<html>
<head>
<title>{{title}}</title>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="https://www.w3schools.com/lib/w3.css">
<link rel="stylesheet" href="https://www.w3schools.com/lib/w3-theme-black.css">
<link rel="stylesheet" href="https://fonts.googleapis.com/css?family=Roboto">
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/4.7.0/css/font-awesome.min.css">
<style>
html,body,h1,h2,h3,h4,h5,h6 {font-family: "Roboto", sans-serif}
.w3-sidenav a,.w3-sidenav h4 {padding: 12px;}
.w3-bar a {
    padding-top: 12px;
    padding-bottom: 12px;
}
</style>
</head>
<body>

    <!-- Navbar -->
    <div class="w3-top">
    <div class="w3-bar w3-theme w3-top w3-left-align w3-large">
        <a class="w3-bar-item w3-button w3-opennav w3-right w3-hide-large w3-hover-white w3-large w3-theme-l1" href="javascript:void(0)" onclick="w3_open()"><i class="fa fa-bars"></i></a>
        <a href="{{ root_dir }}index.html" class="w3-bar-item w3-button w3-theme-l1">Logo/Home</a>
    </div>
    </div>

    <!-- Sidenav -->
    <nav class="w3-sidenav w3-collapse w3-theme-l5 w3-animate-left" style="z-index:3;width:250px;margin-top:51px;" id="mySidenav">
    <a href="javascript:void(0)" onclick="w3_close()" class="w3-right w3-xlarge w3-padding-large w3-hover-black w3-hide-large" title="close menu">
        <i class="fa fa-remove"></i>
    </a>
    <h4><b>Menu</b></h4>
    {% for item in nav_items %}
      {% if item.menu_cmd == "OpenLevel" %}
        <a href="\#" class="w3-deep-orange">{{ item.name }}</a>
      {% else %}
        <a href="{{ root_dir }}{{ item.url }}" class="w3-hover-black">{{ item.name }}</a>
      {% endif %}
    {% endfor %}
    </nav>

    <!-- Overlay effect when opening sidenav on small screens -->
    <div class="w3-overlay w3-hide-large" onclick="w3_close()" style="cursor:pointer" title="close side menu" id="myOverlay"></div>

    <!-- Main content: shift it to the right by 250 pixels when the sidenav is visible -->
    <div class="w3-main" style="margin-left:250px">

    <div class="w3-row w3-padding-64">
        <div class="w3-twothird w3-container">
            {{ content }}
            <p>Click on the image to show enlarge</p>                   
            {% for image in images %}
            <a class="zoom" rel="group" href="{{image.name}}">
               <img src="{{image.thumb}}" />
            </a>
            {% endfor %}
        </div>
    </div>

    <footer id="myFooter">
        <div class="w3-container w3-theme-l2 w3-padding-32">
        <h4>Generated with <a href="https://github.com/phideg/neptungen" target="_blank">neptungen</a></h4>
        </div>

        <div class="w3-container w3-theme-l1">
        <p>Powered by <a href="https://www.w3schools.com/w3css/default.asp" target="_blank">w3.css</a></p>
        </div>
    </footer>

    <!-- END MAIN -->
    </div>

    <script>
        // Get the Sidenav
        var mySidenav = document.getElementById("mySidenav");

        // Get the DIV with overlay effect
        var overlayBg = document.getElementById("myOverlay");

        // Toggle between showing and hiding the sidenav, and add overlay effect
        function w3_open() {
            if (mySidenav.style.display === 'block') {
                mySidenav.style.display = 'none';
                overlayBg.style.display = "none";
            } else {
                mySidenav.style.display = 'block';
                overlayBg.style.display = "block";
            }
        }

        // Close the sidenav with the close button
        function w3_close() {
            mySidenav.style.display = "none";
            overlayBg.style.display = "none";
        }    
    </script>

</body>
</html>
    "#.to_owned()
    )
}