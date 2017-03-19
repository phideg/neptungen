# neptungen
Yet another static Website Generator

# Why
Have you ever designed a static website for your club or one of your relatives but you don't want to maintain the contents for them? Well most static website generators are either dedicated to bloggers or they are too complicated to be used by non digital natives. 

Neptungen should be easy to use and minutes to set up even if you aren't an experienced web developer.

But probably the real reason for neptungen was the desire to learn programming in Rust. So over time the code will hopefully get more idiomatic.

# Features
- Completely written in Rust
- CMS based on CommonMark
- Builtin gallery generator
- Builtin FTP synchronization
- Customizable via liquid templates

# Getting started
Create a new root folder for your website
```bash
mkdir my_new_website
```

Each folder beneath that root folder will be separate navigation entry in your page
```bash
cd my_new_website
mkdir nav1
mkdir nav2
mkdir nav3
```

You can now add a pages to your website by creating *.md files.
```bash
cd nav1
touch my_first_page.md
...
```

Open and edit the markdown files with the markdown editor of your choice.
Each folder should only contain one markdown file plus the images you reference in your markdown file.

The final step is to generate the site. Therefore cd in the root dir of your page and run the generator as follows:
```bash
/path/to/your/neptungen_executable/neptungen
``` 

The generated output can typically be found in the `_output` directory.

# Galleries
Galleries are as simple as other pages. Create a sub directory which must be named `gallery`. Copy or symlink all relevant images into the directory. Finally invoke the generator again as already described before.

The sizes of the images and their thumbs can be controlled via the config.toml configuration file. See section Customize your website.

# Customize your website
You don't want to use the builtin website theme? Create the new file config.toml in the root folder of your project.

TODO: explain config parameters




