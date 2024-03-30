### hive_mind
Minimal simplified shared memory crate using the libc crate


#### Debug & Manual intervention
List shared memory:
```bash
ipcs -m
```

Remove shared memory:
```bash
ipcrm -m <shmid>
```
