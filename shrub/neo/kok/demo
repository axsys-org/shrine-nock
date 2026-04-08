/@  time  /std/pro/time@1
/-  library  /std/lib/test-library
/;    A voting application
/;  This application maintains an index of votes on a particular question
;!  /std/slot/content   /std/pro/text  +
;!  /std/slot/active    /std/pro/flag  +
;!  /std/slot/deadline  /std/pro/time  -
/>  /votes/[user=@p]
  ;!  /std/slot/oath     /std/pro/hex   -
  ;!  /std/slot/content  /std/pro/path  ?
  ==
/<  /std/slot/timer
  ;!  /std/slot/next   atom
  />  /[id=@ud]  +  :: writable
    ;!  /std/slot/deadline  /std/pro/time  -
  ==

    
