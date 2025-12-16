# Tui-sh - An terminal launcher program

> " Proudcitvity is key "

## The goal of this program

This program is to launch various shells in an less paintfull way, to do this , we can use aliases and a little configuration

## Installation

### Windows
Okay , this section is comming first becouse windows is simply more popular

So run the following command:
```ps1
wsl --install
```

This installs WSL , that is basically linux , now after the setup , open WSL and then follow the steps below in the WSL envoriment , essentially , you are on linux now

## Linux/MacOS

The most important part , let me show you step-by-step:

First off , type this command(no brew for mac users sorry):
```bash
git clone https://github.com/mintybrackettemp-hub/tui-sh
```

Okay , now you've cloned the repo to `~` or your home dir

Now type this command:
```bash
cd ~/tui-sh
```
This changes the directory to the repoistory

Finnaly, run the script:
```
./tuish
```

## App

Obviously , you want a guide to use the app

Okay , once you're in the app , you're gonna see the following:

[img](!https://github.com/mintybrackettemp-hub/tui-sh/blob/main/Screenshot%20from%202025-12-16%2015-51-54.png)

Now i'm gonna explain how this works in-depth later , for now , make sure you are seeing this at first launch

### Controls

Use `up/down arrows` to navigate around in the `Actions` section , to switch between `Aliases` and `Actions` section , press `Tab`, then press `enter` to execute the action/functions

### Aliases

Aliases stay permanent becouse of the config file , we're gonna show in-depth detail later , but in the actions section , select `Add an alias`

You want to type the name and command and a keybind , the command is purely shell command , you should learn shell commands to understand what that is, by default , it runs those shell commands on `bash`, a default shell for linux/mac

And boom! the name and keybind will be stored at the `Aliases` section , now `Tab` has an use!

and same applies to `Remove an alias` and `Edit an alias` , it was meant to be user-friendly , so except to learn by the names alone, once runned an alias , you can press any key to exit it

### Shells

This program was meant to be runned as  default shell , so select the `Go to shell` option in the `Actions`, now you're in the interface of that shell, you can type `exit` to go back

And there is also an `Exit shell` option , this essentially quits the shell , like it says

## In-depth details

This is heavily optional , but if you want , you can!

When you run the shell , the shell does some stuff obviously:
- It first checks if there is a config file(located at `~/.config/tuish/cnfg.json`)
- if there is already an config file , it keeps it
- or else , it makes a new one by first launch

You can delete the file to completly reset the program, here is the contents of that config on first launch:
```json
{
  "aliases": {

  },
  "default-shell": "/bin/bash"
}
```

Let's take a look at this , let's break it down

```json
"aliases": {

},
```

This essentially tells the program the aliases properties, and stuff, this part tells there is no aliases, when you make a new shell , this part suddently becomes:

```json
"aliases": {
    "Example Shell": {
      "command": "echo 'this is an example command'",
      "keybind": null
    }
  },
```

Let's break this down obviously:

```json
"Example Shell": {
      "command": "echo 'this is an example command'",
      "keybind": null
    }
```

The `"Example Shell"` Part tells the name of that shell , in this case , Example Shell , now what about the `command` part? you guessed it , the command
same with the `keybind` but it tells the keybind, if the value is `null`, that means there is NO keybind

now going back , What about this part?

```json
"default-shell": "/bin/bash"
```

This tells the default shell , in this case bash , you can change it to tell what shell to use, and you can change any value, this helps keep the aliases permanent


## Help

If the installation is failing , it could be one of the cases:

#### [E01] `git not found` or similar

This means git is not installed , try to install it and then run the command git , if it works , you can safely follow steps

#### [E02] File not found

Make sure you run `pwd` to make sure you are in the repoistory directory, `~/tui-sh`, if not , cd into it
