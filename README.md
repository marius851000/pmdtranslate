# pmdtranslate
pmdtranslate is a tool that allow translation of pokemon super mystery dungeon (at least US/EU rom). It may work with no or minor change on other language or with gates to infinite and RTDX, but I haven't tested this.

## how to use
This is a command line tool. Right now, I don't provide precompiled binary. I guess you will have to look on how to compile rust program (if cargo is already installed, run ``cargo build --release``).

### extract translation
You need to have a decrypted and depacked rom. You may use ``ctrtool`` to unpack a rom. You'll then need to find the file you'll translate. For US PSMD, it should be ``message_us.bin``. Be aware that there should be a ``message_us.lst`` file in the same folder, as it used by the tool.
Then, you'll need to run ``pmdtranslate farc to-pot <path to message_us.bin> <out .pot file>``.

The pot file is the **model** file. You should then use some method to edit ``po`` file (I used poedit).

Make sure the software keep comments and other metadata, as the comment entry is also needed by ``pmdtranslate``.

Once you start editing the string, the input message are in the form of ``<id> text``. When you translate, you should no include the ``<id>`` as well as the next text. For example, if I have ``1014321 Welcome`` and I want to translate it to french, I should write ``Bonjour``.

There may also have special symbol like ``[CENTER]`` or ``[PARTNERNAME]``. Those are special content that shouldn't be translated. If you want to write a ``[``, you need to write ``\[``, and to write a ``\``, you need to write ``\\``. For example, if I want to display ``[HELLO]`` on the screen (rather than having the effect of this character), I would write ``\[HELLO]``. (the ``]`` doesn't need a ``\``).

In addition, you can add extra strings (case insensitive) when calling the program after the parameters so phrase containing it will be differentiated. When translating a file containing "ŧdiscriminatorŧ", don't include anything after the first ŧ.

### use translation in game
First, you'll need a way to patch the game. One cool trick about PSMD is that the game include the functionality to read custom translation (but not custom font) from the SD card. To do this, just place your custom ``message_us.bin`` into the ``private/Nintendo 3DS/app`` folder on the sdcard (create it if needed).

To create a new ``message_us.bin`` file to translate the game, you'll need to run :
``pmdtranslate farc from-po <source .po or .pot file> <target message_us.bin file>``. You should then have the target ``message_us.bin``, that you should load in the game (by placing it at ``private/Nintendo 3DS/app/message_us.bin`` on the sdcard).

You can also patch the file using more traditional patching mathod.

### change font
You'll need to use [pmdfonttool](https://github.com/marius851000/pmdfonttool).