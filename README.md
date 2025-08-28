This is a project I use to skip ads on spotify. It keeps track of a list of all the names of ads on Spotify, and if ever it encounters one, restarts the program, which skips the ad.

There is a small tui component to this that appears as

```
[0.6] Spotify Free
[y/N]
```

where the number is a countdown to when it will next read the title of the Spotify app, the string afterwords being the current title of the Spotify app, and the prompt to be whether or not to add the name to the blacklist.
