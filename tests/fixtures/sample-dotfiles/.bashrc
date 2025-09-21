# ~/.bashrc: executed by bash(1) for non-login shells.

# User configuration
export USER="@user@"
export EMAIL="@email@"
export EDITOR="@editor@"
export SHELL="@shell@"

# Theme settings
export THEME="@theme@"
if [ "$THEME" = "dark" ]; then
  export LS_COLORS="di=34:ln=35:so=32:pi=33:ex=31:bd=34;46:cd=34;43:su=30;41:sg=30;46:tw=30;42:ow=30;43"
fi

# Hostname prompt
PS1='\u@\h:\w\$ '
export PS1

# Aliases
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'
alias grep='grep --color=auto'
alias fgrep='fgrep --color=auto'
alias egrep='egrep --color=auto'

# History settings
HISTCONTROL=ignoreboth
HISTSIZE=1000
HISTFILESIZE=2000

# Custom functions
function mkcd() {
  mkdir -p "$1" && cd "$1"
}

# Machine-specific settings
echo "Welcome to @hostname@, @user@!"
