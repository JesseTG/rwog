NAME
====

rwog - *r*un *w*ith*o*ut *g*roups

SYNOPSIS
========

rwog -g &lt;groups&gt;... \[-- *command-with-args*...\]

DESCRIPTION
===========

**rwog** lets you run a given command while temporarily reducing your group membership. It does not modify `/etc/group` or `/etc/passwd`, and cannot grant you permissions you don't already have. Possible use cases for `rwog` include:

-   In a shared system for which you are a privileged user, pretending that you are an unprivileged user without logging in as one.
-   Testing a program's behavior when it doesn't have the group memberships it needs.

OPTIONS
=======

**-h**, **--help**  
Display the help.

**-g**, **--groups**  
Run the given command without these groups, given by name (not number). You cannot drop your primary group membership (which is output by `id -gn`). Groups that don't exit or that you're not already a member of are ignored.

SEE ALSO
========

`id`(1), `getent`(1), `groups`(1), `group`(5)

BUGS
====

-   Does not support `gid`s given by number. When it does, such `gid`s will be given of the form *`+gid_number`*, as is the case with most `coreutils` programs.

CAVEATS
=======

`rwog` must have the capability `CAP_SETGID` in order to be used. Grant it with `setcap $(which rwog) cap_setgid=pe` if your package manager hasn't done so already. You could run it as root, but given that `rwog` is supposed to *reduce* privileges you'd be missing the point entirely.

I cannot promise that `rwog` is entirely secure. I'm not doing anything blatantly wrong, but it's possible that there's something I missed. **Do not let untrusted users run `rwog`.**

LICENSE
=======

MIT.
