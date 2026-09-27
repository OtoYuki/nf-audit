
process BUSY {
    tag "$id"
    cpus 4
    memory '2 GB'
    time '1h'
    input: val id
    output: val id
    script:
    """
    end=\$((SECONDS+4)); while [ \$SECONDS -lt \$end ]; do :; done
    head -c 150000000 /dev/zero | tail > /dev/null
    """
}

process ESCALATES {
    tag "$id"
    cpus 2
    memory { 1.GB * task.attempt }
    time '30m'
    errorStrategy { task.exitStatus == 137 ? 'retry' : 'terminate' }
    maxRetries 2
    input: val id
    output: val id
    script:
    """
    if [ ${task.attempt} -eq 1 ] && [ "$id" = "s1" ]; then exit 137; fi
    end=\$((SECONDS+2)); while [ \$SECONDS -lt \$end ]; do :; done
    """
}

workflow SUB {
    take: ids
    main:
    BUSY(ids)
    ESCALATES(BUSY.out)
}

workflow NFCORE_DEMO {
    SUB(Channel.of('s1', 's2', 's3'))
}

workflow {
    NFCORE_DEMO()
}
