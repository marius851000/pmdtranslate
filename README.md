# pmdtranslate
pmdtranslate is a tool that allow translation of pokemon super mystery dungeon (us rom). It may work with no or minor change on other language or with gates to infinite, but I haven't tested this.

## how to use
This is a command line tool. Right now, I don't provide precompiled binary. I guess you will have to look on how to compile rust program (if cargo is already installed, run ``cargo build --release``).

### extract translation
You need to have a decrypted and depacked rom. You may use ``ctrtool`` to unpack a rom. You'll then need to find the file you'll translate. For US PSMD, it should be ``message_us.bin``. Be aware that there should be a ``message_us.lst`` file in the same folder, as it used by the tool.
Then, you'll need to run ``pmdtranslate farc to-pot <path to message_us.bin> <out .pot file>``.

The pot file is the **model** file. You should then use some method to edit ``po`` file (I used poedit).

Make sure the software keep comments and other metadata, as the comment entry is also needed by ``pmdtranslate``.

Once you start editing the string, the input message are in the form of ``<id> text``. When you translate, you should no include the ``<id>`` as well as the next text. For example, if I have ``1014321 Welcome`` and I want to translate it to french, I should write ``Bonjour``.

There may also have special symbol like ``[CENTER]`` or ``[PARTNERNAME]``. Those are special content that should'nt be translated. If you want to write a ``[``, you need to write ``\[``, and to write a ``\``, you need to write ``\\``. For example, if I want to display ``[HELLO]`` on the screen (rather than having the effect of this character), I would write ``\[HELLO]``. (the ``]`` doesn't need a ``\``).

### use translation in game
First, you'll need a way to patch the game. This is outside of the scope of this tutorial, but here are still some information: The US romid of PSMD is ``0004000000174600``, and I recommend either luma if you want to test on a real 3ds, or the mod function of the citra emulator.

To create a new ``message_us.bin`` file to translate the game, you'll need to run :
``pmdtranslate farc from-po <source .po or .pot file> <target message_us.bin file>``. You should then have the target ``message_us.bin``, that you should load in the game (replace the ``message_us.bin`` file via the patch tool).

### change font
You'll need to use [pmdfonttool](https://github.com/marius851000/pmdfonttool). I need to test the possibility to properly adde new character.